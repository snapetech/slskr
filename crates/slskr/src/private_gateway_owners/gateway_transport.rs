use super::*;

#[allow(dead_code, clippy::too_many_arguments)]
impl Gateway {
    pub async fn load_or_create_with_quic(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data(bind, state_dir, quic_bind, None).await
    }

    pub async fn load_or_create_with_quic_and_data(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            None,
        )
        .await
    }

    pub async fn load_or_create_with_quic_and_data_policy(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            None,
            quic_data_policy,
            slskr_client::quic_data::DEFAULT_MAX_CONCURRENT_STREAMS as usize,
        )
        .await
    }

    pub async fn load_or_create_with_quic_and_data_policy_and_proxy(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy_inner(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            quic_proxy_bind,
            None,
            None,
            None,
            quic_data_policy,
            max_concurrent_streams,
            false,
            false,
        )
        .await
    }

    /// Construct a gateway whose public UDP listener also owns a configured
    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
    /// endpoint; overlay and QUIC traffic remains handled by this gateway.
    pub async fn load_or_create_with_quic_and_data_policy_and_proxy_and_dht(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        shared_udp_bind: Option<SocketAddr>,
        dht_backend: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy_inner(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            quic_proxy_bind,
            shared_udp_bind,
            None,
            dht_backend,
            quic_data_policy,
            max_concurrent_streams,
            false,
            false,
        )
        .await
    }

    /// Construct a gateway using a socket already bound by the shared DHT
    /// runtime. Mainline uses another clone for outbound packets, while the
    /// gateway owns the Tokio receive/send half for overlay demultiplexing.
    pub async fn load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        shared_udp_bind: Option<SocketAddr>,
        shared_udp_socket: Option<Arc<StdUdpSocket>>,
        dht_backend: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy_inner(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            quic_proxy_bind,
            shared_udp_bind,
            shared_udp_socket,
            dht_backend,
            quic_data_policy,
            max_concurrent_streams,
            false,
            false,
        )
        .await
    }

    pub async fn load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        shared_udp_bind: Option<SocketAddr>,
        shared_udp_socket: Option<Arc<StdUdpSocket>>,
        dht_backend: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
        data_shared_with_dht: bool,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy_inner(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            quic_proxy_bind,
            shared_udp_bind,
            shared_udp_socket,
            dht_backend,
            quic_data_policy,
            max_concurrent_streams,
            data_shared_with_dht,
            false,
        )
        .await
    }

    /// Construct a gateway whose TLS TCP connections arrive through the
    /// application's shared Soulseek/mesh listener. The gateway still owns
    /// its UDP and QUIC services, but does not bind a second TCP socket.
    pub async fn load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share_shared_tcp(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        shared_udp_bind: Option<SocketAddr>,
        shared_udp_socket: Option<Arc<StdUdpSocket>>,
        dht_backend: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
        data_shared_with_dht: bool,
    ) -> Result<Self, String> {
        Self::load_or_create_with_quic_and_data_policy_and_proxy_inner(
            bind,
            state_dir,
            quic_bind,
            quic_data_bind,
            quic_proxy_bind,
            shared_udp_bind,
            shared_udp_socket,
            dht_backend,
            quic_data_policy,
            max_concurrent_streams,
            data_shared_with_dht,
            true,
        )
        .await
    }

    pub(super) async fn load_or_create_with_quic_and_data_policy_and_proxy_inner(
        bind: SocketAddr,
        state_dir: &Path,
        quic_bind: Option<SocketAddr>,
        quic_data_bind: Option<SocketAddr>,
        quic_proxy_bind: Option<SocketAddr>,
        shared_udp_bind: Option<SocketAddr>,
        shared_udp_socket: Option<Arc<StdUdpSocket>>,
        dht_backend: Option<SocketAddr>,
        quic_data_policy: Option<QuicDataPolicy>,
        max_concurrent_streams: usize,
        data_shared_with_dht: bool,
        shared_tcp: bool,
    ) -> Result<Self, String> {
        let (certificate, private_key) = load_or_create_certificate(state_dir)?;
        let certificate_sha256 = Sha256::digest(certificate.as_ref()).into();
        let config =
            ServerConfig::builder_with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], private_key.clone_key().into())
                .map_err(|error| format!("overlay TLS configuration failed: {error}"))?;
        let quic_listener = quic_bind.and_then(|bind| {
            match QuicControlServer::bind(bind, certificate.clone(), private_key.clone_key()) {
                Ok(listener) => Some(listener),
                Err(error) => {
                    tracing::warn!(%error, ?bind, "overlay QUIC control listener unavailable");
                    None
                }
            }
        });
        let quic_data_listener = quic_data_bind.and_then(|bind| {
            match QuicDataServer::bind_with_limits(
                bind,
                certificate,
                private_key,
                QUIC_DATA_MAX_PAYLOAD_BYTES,
                u32::try_from(max_concurrent_streams).unwrap_or(u32::MAX),
            ) {
                Ok(listener) => Some(listener),
                Err(error) => {
                    tracing::warn!(%error, ?bind, "overlay QUIC data listener unavailable");
                    None
                }
            }
        });
        let listener = if shared_tcp {
            None
        } else {
            Some(
                TcpListener::bind(bind)
                    .await
                    .map_err(|error| format!("overlay listener bind failed: {error}"))?,
            )
        };
        let bind = listener
            .as_ref()
            .map(TcpListener::local_addr)
            .transpose()
            .map_err(|error| format!("overlay listener address failed: {error}"))?
            .unwrap_or(bind);
        // native profile's UDP control plane normally shares its public socket with
        // DHT.  The public gateway owns that socket in shared mode and sends
        // only DHT-shaped datagrams to mainline's internal endpoint.
        let udp_listener = if let Some(shared_socket) = shared_udp_socket {
            let socket = shared_socket
                .try_clone()
                .map_err(|error| format!("shared UDP socket clone failed: {error}"))?;
            socket
                .set_nonblocking(true)
                .map_err(|error| format!("shared UDP socket nonblocking setup failed: {error}"))?;
            Some(
                UdpSocket::from_std(socket)
                    .map_err(|error| format!("shared UDP Tokio socket setup failed: {error}"))?,
            )
        } else {
            let udp_bind = shared_udp_bind.or(quic_proxy_bind).unwrap_or(bind);
            match UdpSocket::bind(udp_bind).await {
                Ok(socket) => Some(socket),
                Err(error) => {
                    tracing::debug!(%error, ?udp_bind, "overlay UDP control listener unavailable");
                    None
                }
            }
        };
        let dht_forward_socket = if dht_backend.is_some() {
            Some(Arc::new(
                UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))
                    .await
                    .map_err(|error| format!("DHT forwarding socket bind failed: {error}"))?,
            ))
        } else {
            None
        };
        Ok(Self {
            bind: StdRwLock::new(bind),
            acceptor: TlsAcceptor::from(Arc::new(config)),
            certificate_sha256,
            listener: Mutex::new(listener),
            udp_listener: Mutex::new(udp_listener),
            dht_forward_socket,
            dht_forward_target: dht_backend,
            quic_listener: Mutex::new(quic_listener),
            quic_data_listener: Mutex::new(quic_data_listener),
            quic_proxy_backend: quic_proxy_bind
                .zip(quic_bind)
                .filter(|(_, backend)| backend.ip().is_loopback())
                .map(|(_, backend)| backend),
            quic_data_proxy_backend: data_shared_with_dht.then_some(quic_data_bind).flatten(),
            quic_data_max_concurrent_streams: max_concurrent_streams.clamp(1, 1_024),
            quic_data_relays: Arc::new(Semaphore::new(
                quic_data_policy
                    .as_ref()
                    .map_or(1, |policy| policy.max_concurrent_relays.max(1)),
            )),
            quic_data_policy: quic_data_policy.map(Arc::new),
            connections: Arc::new(Semaphore::new(MAX_GATEWAY_CONNECTIONS)),
            tunnels: RwLock::new(BTreeMap::new()),
            overlay_connections: RwLock::new(BTreeMap::new()),
            replay_nonces: Mutex::new(BTreeMap::new()),
            overlay_rate_limiter: Arc::new(OverlayRateLimiter::new()),
            dht_service: crate::mesh_dht::DhtServiceState::default(),
        })
    }

    #[must_use]
    pub const fn certificate_sha256(&self) -> [u8; 32] {
        self.certificate_sha256
    }

    #[must_use]
    pub fn bind(&self) -> SocketAddr {
        *self
            .bind
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn set_bind(&self, bind: SocketAddr) {
        *self
            .bind
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = bind;
    }

    /// Dispatch an already-accepted TLS connection through the gateway's
    /// normal rate limits, connection semaphore, and handshake handler.
    /// Shared TCP ownership lives in the Soulseek listener manager; this
    /// method keeps all overlay admission behavior in one place.
    pub(crate) async fn handle_accepted_tcp(
        self: &Arc<Self>,
        tcp: TcpStream,
        state: Arc<crate::AppState>,
    ) {
        let remote_address = match tcp.peer_addr() {
            Ok(address) => address,
            Err(error) => {
                tracing::debug!(%error, "overlay shared TCP peer address unavailable");
                return;
            }
        };
        let remote_ip = remote_address.ip();
        if !self
            .overlay_rate_limiter
            .check_connection(remote_ip)
            .allowed
        {
            return;
        }
        let Ok(permit) = Arc::clone(&self.connections).try_acquire_owned() else {
            self.overlay_rate_limiter.record_disconnection(remote_ip);
            return;
        };
        let gateway = Arc::clone(self);
        let admission = GatewayConnectionAdmission {
            limiter: Arc::clone(&self.overlay_rate_limiter),
            remote_ip,
        };
        let registry_state = Arc::clone(&state);
        registry_state.managed_background_tasks.spawn(async move {
            let _permit = permit;
            let _admission = admission;
            if let Err(error) = gateway.handle_connection(tcp, &state).await {
                tracing::debug!(%error, "overlay gateway connection closed");
            }
        });
    }

    /// Real count of currently-open overlay tunnels -- backs the
    /// oracle's `ServerStatsResponse.ActiveConnections`-style fields,
    /// which several HTTP routes previously hardcoded to 0 despite this
    /// registry already tracking real, live connections.
    pub async fn active_connection_count(&self) -> usize {
        self.tunnels.read().await.len()
    }

    /// Return metadata for currently-open, authenticated TLS overlay sessions.
    pub async fn active_overlay_connections(&self) -> Vec<OverlayConnectionMetadata> {
        self.overlay_connections
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub async fn register_outbound_overlay(
        &self,
        username: String,
        endpoint: SocketAddr,
        features: Vec<String>,
        version: i32,
        certificate_thumbprint: Option<String>,
    ) -> Result<String, String> {
        if username.trim().is_empty()
            || username.len() > MAX_OVERLAY_METADATA_USERNAME_BYTES
            || username.chars().any(char::is_control)
            || features.len() > MAX_OVERLAY_METADATA_FEATURES
            || features.iter().any(|feature| {
                feature.is_empty()
                    || feature.len() > MAX_OVERLAY_METADATA_FEATURE_BYTES
                    || feature.chars().any(char::is_control)
            })
            || certificate_thumbprint.as_deref().is_some_and(|thumbprint| {
                thumbprint.is_empty()
                    || thumbprint.len() > MAX_OVERLAY_METADATA_THUMBPRINT_BYTES
                    || thumbprint.chars().any(char::is_control)
            })
        {
            return Err("outbound overlay metadata is invalid".to_owned());
        }
        let mut connections = self.overlay_connections.write().await;
        if connections.len() >= MAX_GATEWAY_CONNECTIONS {
            return Err("overlay connection capacity is full".to_owned());
        }
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        let timestamp = overlay_timestamp();
        connections.insert(
            connection_id.clone(),
            OverlayConnectionMetadata {
                username,
                address: endpoint.ip().to_string(),
                port: endpoint.port(),
                features,
                connected_at: timestamp.clone(),
                last_activity: timestamp,
                certificate_thumbprint,
                version,
                is_outbound: true,
            },
        );
        Ok(connection_id)
    }

    pub async fn remove_overlay_connection(&self, connection_id: &str) {
        self.overlay_connections.write().await.remove(connection_id);
    }

    pub async fn register_outbound_guard(
        self: &Arc<Self>,
        username: String,
        endpoint: SocketAddr,
        features: Vec<String>,
        version: i32,
        certificate_thumbprint: Option<String>,
    ) -> Result<OutboundOverlayGuard, String> {
        let connection_id = self
            .register_outbound_overlay(
                username,
                endpoint,
                features,
                version,
                certificate_thumbprint,
            )
            .await;
        Ok(OutboundOverlayGuard {
            gateway: Arc::clone(self),
            connection_id: connection_id?,
        })
    }

    pub async fn run(self: Arc<Self>, state: Arc<crate::AppState>) -> Result<(), String> {
        let listener = self.listener.lock().await.take();
        if let Some(udp_listener) = self.udp_listener.lock().await.take() {
            let gateway = Arc::clone(&self);
            let udp_state = Arc::clone(&state);
            let quic_proxy_backend = self.quic_proxy_backend;
            let quic_data_proxy_backend = self.quic_data_proxy_backend;
            state.managed_background_tasks.spawn(async move {
                gateway
                    .run_udp_control(
                        udp_listener,
                        udp_state,
                        quic_proxy_backend,
                        quic_data_proxy_backend,
                    )
                    .await;
            });
        }
        if let Some(quic_listener) = self.quic_listener.lock().await.take() {
            let gateway = Arc::clone(&self);
            let quic_state = Arc::clone(&state);
            state.managed_background_tasks.spawn(async move {
                gateway.run_quic_control(quic_listener, quic_state).await;
            });
        }
        if let Some(quic_data_listener) = self.quic_data_listener.lock().await.take() {
            let gateway = Arc::clone(&self);
            let quic_state = Arc::clone(&state);
            state.managed_background_tasks.spawn(async move {
                gateway.run_quic_data(quic_data_listener, quic_state).await;
            });
        }
        let Some(listener) = listener else {
            // Shared TCP mode leaves the accept loop to the Soulseek listener
            // manager. UDP/QUIC workers above are still owned by this gateway.
            return Ok(());
        };
        loop {
            let (tcp, _) = listener
                .accept()
                .await
                .map_err(|error| format!("overlay listener accept failed: {error}"))?;
            self.handle_accepted_tcp(tcp, Arc::clone(&state)).await;
        }
    }

    pub(super) async fn run_udp_control(
        &self,
        socket: UdpSocket,
        state: Arc<crate::AppState>,
        quic_proxy_backend: Option<SocketAddr>,
        quic_data_proxy_backend: Option<SocketAddr>,
    ) {
        let public_socket = Arc::new(socket);
        let mut quic_sessions = HashMap::new();
        let quic_admission = QuicProxyAdmissionGate::default();
        if let Some(forward_socket) = self.dht_forward_socket.as_ref() {
            state.managed_background_tasks.spawn(forward_dht_responses(
                Arc::clone(forward_socket),
                Arc::clone(&public_socket),
            ));
        }
        let mut buffer = [0_u8; 65_536];
        loop {
            prune_quic_proxy_sessions(&mut quic_sessions);
            let received = match public_socket.recv_from(&mut buffer).await {
                Ok(received) => received,
                Err(error) => {
                    tracing::debug!(%error, "overlay UDP control listener stopped");
                    return;
                }
            };
            if let Some(session) = quic_sessions.get_mut(&received.1) {
                session
                    .last_activity
                    .store(crate::unix_timestamp(), Ordering::Relaxed);
                if let Err(error) = session.sender.try_send(buffer[..received.0].to_vec()) {
                    if matches!(error, mpsc::error::TrySendError::Closed(_)) {
                        quic_sessions.remove(&received.1);
                    }
                }
                continue;
            }
            if is_dht_packet(&buffer[..received.0]) {
                if let (Some(forward_socket), Some(forward_target)) =
                    (&self.dht_forward_socket, self.dht_forward_target)
                {
                    if let Err(error) = forward_socket
                        .send_to(&buffer[..received.0], forward_target)
                        .await
                    {
                        tracing::debug!(%error, ?forward_target, "shared DHT datagram forwarding failed");
                    }
                }
                // DHT traffic is never handed to the overlay decoder. In
                // standalone mode there is no forward target, so it is
                // intentionally ignored just as before.
                continue;
            }
            if is_quic_initial_packet(&buffer[..received.0]) {
                let Some(quic_backend) = select_quic_proxy_backend(
                    &buffer[..received.0],
                    quic_proxy_backend,
                    quic_data_proxy_backend,
                ) else {
                    continue;
                };
                if quic_sessions.len() >= QUIC_PROXY_MAX_SESSIONS {
                    continue;
                }
                let Some(admission_lease) = quic_admission.try_acquire(received.1) else {
                    continue;
                };
                if let Ok(session) = QuicProxySession::new(
                    received.1,
                    quic_backend,
                    Arc::clone(&public_socket),
                    admission_lease,
                )
                .await
                {
                    if session
                        .sender
                        .send(buffer[..received.0].to_vec())
                        .await
                        .is_ok()
                    {
                        quic_sessions.insert(received.1, session);
                    } else {
                        tracing::debug!(
                            remote = ?received.1,
                            "overlay QUIC proxy closed before initial datagram was forwarded"
                        );
                    }
                }
                continue;
            }
            if !self
                .overlay_rate_limiter
                .check_message(&overlay_datagram_limiter_id(received.1))
                .allowed
            {
                continue;
            }
            let Ok(envelope) = ControlEnvelope::decode(&buffer[..received.0]) else {
                continue;
            };
            let now = match i64::try_from(crate::unix_timestamp_millis()) {
                Ok(now) => now,
                Err(_) => continue,
            };
            if !envelope.timestamp_is_current(now) || envelope.verify().is_err() {
                continue;
            }
            if envelope.message_type != "pod_message" {
                // Target ControlDispatcher intentionally ignores unknown
                // control types after decode; retain that one-way behavior.
                continue;
            }
            let Ok(message) = serde_json::from_slice::<PodControlMessage>(&envelope.payload) else {
                continue;
            };
            if message.sender_peer_id.trim().is_empty()
                || message.message_id.trim().is_empty()
                || message.timestamp_unix_ms <= 0
            {
                continue;
            }
            if let Err((status, error)) = self
                .handle_pods_call(
                    "PostMessage",
                    &envelope.payload,
                    message.sender_peer_id.trim(),
                    &state,
                )
                .await
            {
                tracing::warn!(
                    %status,
                    %error,
                    sender_peer_id = message.sender_peer_id.trim(),
                    "overlay pod message dispatch failed"
                );
            }
        }
    }

    pub(super) async fn run_quic_control(
        self: Arc<Self>,
        server: QuicControlServer,
        state: Arc<crate::AppState>,
    ) {
        loop {
            let Some(connection) = server.accept().await else {
                return;
            };
            let connection = match connection {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::debug!(%error, "overlay QUIC control connection rejected");
                    continue;
                }
            };
            let remote_ip = connection.remote_address().ip();
            if !self
                .overlay_rate_limiter
                .check_connection(remote_ip)
                .allowed
            {
                continue;
            }
            let Ok(permit) = Arc::clone(&self.connections).try_acquire_owned() else {
                self.overlay_rate_limiter.record_disconnection(remote_ip);
                continue;
            };
            let gateway = Arc::clone(&self);
            let connection_state = Arc::clone(&state);
            let admission = GatewayConnectionAdmission {
                limiter: Arc::clone(&self.overlay_rate_limiter),
                remote_ip,
            };
            state.managed_background_tasks.spawn(async move {
                let _permit = permit;
                let _admission = admission;
                gateway
                    .handle_quic_connection(connection, connection_state)
                    .await;
            });
        }
    }

    pub(super) async fn run_quic_data(
        self: Arc<Self>,
        server: QuicDataServer,
        state: Arc<crate::AppState>,
    ) {
        loop {
            let Some(connection) = server.accept().await else {
                return;
            };
            let connection = match connection {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::debug!(%error, "overlay QUIC data connection rejected");
                    continue;
                }
            };
            let remote_ip = connection.remote_address().ip();
            if !self
                .overlay_rate_limiter
                .check_connection(remote_ip)
                .allowed
            {
                continue;
            }
            let Ok(permit) = Arc::clone(&self.connections).try_acquire_owned() else {
                self.overlay_rate_limiter.record_disconnection(remote_ip);
                continue;
            };
            let gateway = Arc::clone(&self);
            let admission = GatewayConnectionAdmission {
                limiter: Arc::clone(&self.overlay_rate_limiter),
                remote_ip,
            };
            state.managed_background_tasks.spawn(async move {
                let _permit = permit;
                let _admission = admission;
                Arc::clone(&gateway)
                    .handle_quic_data_connection(connection)
                    .await;
            });
        }
    }

    pub(super) async fn handle_quic_data_connection(
        self: Arc<Self>,
        connection: QuicDataConnection,
    ) {
        let remote = connection.remote_address();
        let stream_permits = Arc::new(Semaphore::new(self.quic_data_max_concurrent_streams));
        let mut stream_tasks = JoinSet::new();
        loop {
            let stream =
                match timeout(QUIC_DATA_READ_TIMEOUT, connection.accept_inbound_stream()).await {
                    Err(_) => {
                        tracing::debug!(?remote, "overlay QUIC data connection read timed out");
                        break;
                    }
                    Ok(Ok(stream)) => stream,
                    Ok(Err(QuicDataError::Connection(error))) => {
                        tracing::debug!(%error, ?remote, "overlay QUIC data connection closed");
                        break;
                    }
                    Ok(Err(error)) => {
                        tracing::debug!(%error, ?remote, "overlay QUIC data stream rejected");
                        continue;
                    }
                };
            let Ok(permit) = Arc::clone(&stream_permits).acquire_owned().await else {
                break;
            };
            let gateway = Arc::clone(&self);
            stream_tasks.spawn(async move {
                let _permit = permit;
                match stream {
                    QuicDataInboundStream::Bidirectional(stream) => {
                        gateway.handle_quic_data_stream(stream, remote).await;
                    }
                    QuicDataInboundStream::Unidirectional(mut receive) => {
                        match timeout(QUIC_DATA_READ_TIMEOUT, receive.read_to_end()).await {
                            Ok(Ok(payload)) => tracing::debug!(
                                size = payload.len(),
                                ?remote,
                                "received overlay QUIC unidirectional data payload"
                            ),
                            Ok(Err(error)) => tracing::debug!(
                                %error,
                                ?remote,
                                "overlay QUIC unidirectional data payload rejected"
                            ),
                            Err(_) => tracing::debug!(
                                ?remote,
                                "overlay QUIC unidirectional data payload read timed out"
                            ),
                        }
                    }
                }
            });
        }
        while let Some(result) = stream_tasks.join_next().await {
            if let Err(error) = result {
                tracing::warn!(%error, ?remote, "overlay QUIC data stream task failed");
            }
        }
    }

    pub(super) async fn handle_quic_data_stream(
        &self,
        stream: slskr_client::quic_data::QuicDataStream,
        remote: SocketAddr,
    ) {
        let (mut send, mut receive) = stream.split();
        let (line, line_bytes) = match read_quic_data_command_line_with_timeout(&mut receive).await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::debug!(%error, ?remote, "overlay QUIC data command rejected");
                return;
            }
        };

        if line.starts_with("RELAY_TCP ") {
            let _ = write_quic_data_error(&mut send, "authentication required").await;
            return;
        }

        if line.starts_with("AUTH ") {
            let Some(policy) = self.quic_data_policy.as_ref() else {
                let _ = write_quic_data_error(&mut send, "relay disabled").await;
                return;
            };
            if !relay_authentication_valid(&line, &policy.relay_authentication_token) {
                let _ = write_quic_data_error(&mut send, "authentication failed").await;
                return;
            }
            let relay_line = match read_quic_data_command_line_with_timeout(&mut receive).await {
                Ok((line, _)) => line,
                Err(_) => {
                    let _ = write_quic_data_error(&mut send, "bad command").await;
                    return;
                }
            };
            let parts = relay_line.split(' ').collect::<Vec<_>>();
            let Some((_, host, port)) =
                (parts.len() == 3).then(|| (parts[0], parts[1], parts[2].parse::<u16>().ok()))
            else {
                let _ = write_quic_data_error(&mut send, "bad command").await;
                return;
            };
            let Some(port) = port else {
                let _ = write_quic_data_error(&mut send, "bad command").await;
                return;
            };
            if parts[0] != "RELAY_TCP" {
                let _ = write_quic_data_error(&mut send, "bad command").await;
                return;
            }
            if !allowed_relay_destination(policy, host, port) {
                let _ = write_quic_data_error(&mut send, "destination denied").await;
                return;
            }
            let destination = match resolve_public_relay_destination(host, port).await {
                Ok(destination) => destination,
                Err(_) => {
                    let _ = write_quic_data_error(&mut send, "destination denied").await;
                    return;
                }
            };
            let Ok(permit) = Arc::clone(&self.quic_data_relays).try_acquire_owned() else {
                let _ = write_quic_data_error(&mut send, "relay capacity reached").await;
                return;
            };
            let tcp =
                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
                    Ok(Ok(tcp)) => tcp,
                    _ => {
                        let _ = write_quic_data_error(&mut send, "relay failed").await;
                        drop(permit);
                        return;
                    }
                };
            if timeout(DESTINATION_WRITE_TIMEOUT, send.write_all(b"OK\n"))
                .await
                .is_err()
            {
                drop(permit);
                return;
            }
            let (tcp_read, tcp_write) = tcp.into_split();
            let max_bytes = policy.max_relay_bytes_per_direction.max(1);
            let relay = async {
                tokio::select! {
                    result = copy_quic_to_tcp(receive, tcp_write, max_bytes) => result,
                    result = copy_tcp_to_quic(tcp_read, send, max_bytes) => result,
                }
            };
            match timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::debug!(%error, ?remote, "overlay QUIC relay stopped with an error");
                }
                Err(_) => {
                    tracing::warn!(
                        ?remote,
                        "overlay QUIC relay exceeded its configured duration"
                    );
                }
            }
            drop(permit);
            return;
        }

        let remaining = match timeout(
            QUIC_DATA_READ_TIMEOUT,
            receive.read_to_end_after(line_bytes.len()),
        )
        .await
        {
            Ok(Ok(remaining)) => remaining,
            Ok(Err(error)) => {
                tracing::debug!(%error, ?remote, "overlay QUIC data payload rejected");
                return;
            }
            Err(_) => {
                tracing::debug!(?remote, "overlay QUIC data payload read timed out");
                return;
            }
        };
        tracing::debug!(
            size = line_bytes.len().saturating_add(remaining.len()),
            ?remote,
            "received overlay QUIC data payload"
        );
    }

    pub(super) async fn handle_quic_connection(
        &self,
        connection: QuicControlConnection,
        state: Arc<crate::AppState>,
    ) {
        let remote = connection.remote_address();
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        loop {
            if !self
                .overlay_rate_limiter
                .check_message(&connection_id)
                .allowed
            {
                self.overlay_rate_limiter.remove_connection(&connection_id);
                return;
            }
            let envelope =
                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
                    Ok(Ok(envelope)) => envelope,
                    Ok(Err(QuicControlError::Connection(error))) => {
                        tracing::debug!(%error, ?remote, "overlay QUIC control connection closed");
                        self.overlay_rate_limiter.remove_connection(&connection_id);
                        return;
                    }
                    Ok(Err(error)) => {
                        tracing::debug!(%error, ?remote, "overlay QUIC control stream rejected");
                        continue;
                    }
                    Err(_) => {
                        tracing::debug!(?remote, "overlay QUIC control stream read timed out");
                        continue;
                    }
                };
            let now = match i64::try_from(crate::unix_timestamp_millis()) {
                Ok(now) => now,
                Err(_) => continue,
            };
            if !envelope.timestamp_is_current(now) || envelope.verify().is_err() {
                continue;
            }
            if envelope.message_type != "pod_message" {
                // This preserves the frozen dispatcher behavior for control
                // types that have no local service implementation.
                continue;
            }
            let Ok(message) = serde_json::from_slice::<PodControlMessage>(&envelope.payload) else {
                continue;
            };
            if message.sender_peer_id.trim().is_empty()
                || message.message_id.trim().is_empty()
                || message.timestamp_unix_ms <= 0
            {
                continue;
            }
            if let Err((status, error)) = self
                .handle_pods_call(
                    "PostMessage",
                    &envelope.payload,
                    message.sender_peer_id.trim(),
                    &state,
                )
                .await
            {
                tracing::warn!(
                    %status,
                    %error,
                    sender_peer_id = message.sender_peer_id.trim(),
                    "overlay QUIC pod message dispatch failed"
                );
            }
        }
    }
}
