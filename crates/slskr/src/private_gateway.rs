use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt, fs,
    io::{Read as _, Seek as _, SeekFrom},
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex as StdMutex, RwLock as StdRwLock,
    },
    time::{Duration, Instant},
};

use crate::mesh_security::OverlayRateLimiter;
use crate::quic_alpn;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rcgen::generate_simple_self_signed;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshHelloAck, MeshSearchFileDto,
    MeshSearchRequestMessage, MeshSearchResponseMessage, MeshServiceCall, MeshServiceReply,
    OpenTunnelRequest, OpenTunnelResponse, OverlayFramer, Ping, Pong, TunnelDataRequest,
    TunnelDataResponse, FEATURE_MESH_SEARCH, FEATURE_MESH_SERVICE, MAX_OVERLAY_MESSAGE_BYTES,
    OVERLAY_MAGIC, OVERLAY_VERSION,
};
use slskr_client::overlay_control::ControlEnvelope;
use slskr_client::quic_control::{QuicControlConnection, QuicControlError, QuicControlServer};
use slskr_client::quic_data::{
    QuicDataConnection, QuicDataError, QuicDataInboundStream, QuicDataReceiveStream,
    QuicDataSendStream, QuicDataServer,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, tcp::OwnedWriteHalf, TcpListener, TcpStream, UdpSocket},
    sync::{mpsc, Mutex, RwLock, Semaphore},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{
    rustls::{
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};

const MAX_GATEWAY_CONNECTIONS: usize = 128;
const MAX_TUNNELS: usize = 128;
const MAX_TUNNELS_PER_PEER: usize = 10;
const MAX_REPLAY_NONCES: usize = 4_096;
const MAX_REPLAY_NONCES_PER_PEER: usize = 128;
const MAX_POD_ID_BYTES: usize = 512;
const MAX_DESTINATION_HOST_BYTES: usize = 255;
const MAX_SERVICE_NAME_BYTES: usize = 128;
const MAX_REQUEST_NONCE_BYTES: usize = 64;
const MAX_CERTIFICATE_BYTES: u64 = 64 * 1024;
const MAX_PRIVATE_KEY_BYTES: u64 = 16 * 1024;
const REQUEST_FRESHNESS_SECONDS: u64 = 300;
const DESTINATION_RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const DESTINATION_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DESTINATION_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(2 * 60);
const OVERLAY_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const QUIC_DATA_READ_TIMEOUT: Duration = Duration::from_secs(30);
const INBOUND_BUFFER_CHUNKS: usize = 64;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;
const MAX_POD_MESSAGE_BODY_BYTES: usize = 4 * 1024;
const QUIC_DATA_MAX_PAYLOAD_BYTES: usize = slskr_client::quic_data::DEFAULT_MAX_PAYLOAD_BYTES;
const QUIC_PROXY_MAX_SESSIONS: usize = 128;
const QUIC_PROXY_PREFIX_SESSION_LIMIT: usize = 8;
const QUIC_PROXY_GLOBAL_ATTEMPT_LIMIT: usize = 64;
const QUIC_PROXY_PREFIX_ATTEMPT_LIMIT: usize = 4;
const QUIC_PROXY_ATTEMPT_WINDOW: Duration = Duration::from_secs(10);
const QUIC_PROXY_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const QUIC_PROXY_PENDING_TIMEOUT: Duration = Duration::from_secs(10);

struct OverlayLiveness {
    last_inbound: Instant,
    last_ping: Instant,
}

impl OverlayLiveness {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last_inbound: now,
            last_ping: now,
        }
    }

    fn record_inbound(&mut self) {
        self.last_inbound = Instant::now();
    }

    fn record_ping(&mut self) {
        self.last_ping = Instant::now();
    }

    fn is_idle(&self) -> bool {
        self.last_inbound.elapsed() >= OVERLAY_IDLE_TIMEOUT
    }

    fn read_wait(&self) -> Duration {
        OVERLAY_MESSAGE_READ_TIMEOUT.min(
            OVERLAY_KEEPALIVE_INTERVAL
                .checked_sub(self.last_ping.elapsed())
                .unwrap_or(Duration::ZERO),
        )
    }
}
const MAX_MESH_CONTENT_BYTES: usize = 32 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 512;
const MAX_SHADOW_MBID_BYTES: usize = 100;
const MAX_SHADOW_BATCH: usize = 20;

#[derive(Clone, Debug)]
pub struct QuicDataPolicy {
    pub relay_authentication_token: String,
    pub allowed_relay_destinations: Vec<String>,
    pub max_concurrent_relays: usize,
    pub max_relay_bytes_per_direction: u64,
    pub max_relay_duration: Duration,
}

pub struct Gateway {
    bind: StdRwLock<SocketAddr>,
    acceptor: TlsAcceptor,
    certificate_sha256: [u8; 32],
    listener: Mutex<Option<TcpListener>>,
    udp_listener: Mutex<Option<UdpSocket>>,
    dht_forward_socket: Option<Arc<UdpSocket>>,
    dht_forward_target: Option<SocketAddr>,
    quic_listener: Mutex<Option<QuicControlServer>>,
    quic_data_listener: Mutex<Option<QuicDataServer>>,
    quic_proxy_backend: Option<SocketAddr>,
    quic_data_proxy_backend: Option<SocketAddr>,
    quic_data_policy: Option<Arc<QuicDataPolicy>>,
    quic_data_max_concurrent_streams: usize,
    quic_data_relays: Arc<Semaphore>,
    connections: Arc<Semaphore>,
    tunnels: RwLock<BTreeMap<String, Arc<Tunnel>>>,
    overlay_connections: RwLock<BTreeMap<String, OverlayConnectionMetadata>>,
    replay_nonces: Mutex<BTreeMap<(String, String), u64>>,
    overlay_rate_limiter: Arc<OverlayRateLimiter>,
    dht_service: crate::mesh_dht::DhtServiceState,
}

impl fmt::Debug for Gateway {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Gateway")
            .field(
                "bind",
                &self
                    .bind
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            )
            .field("certificate_sha256", &hex::encode(self.certificate_sha256))
            .finish_non_exhaustive()
    }
}

fn overlay_service_enabled(
    service_name: &str,
    features: &crate::config::FeatureGateSettings,
    target: crate::config::ControllerProfile,
) -> bool {
    // The frozen native profile application only registers DHT, hole-punch, and
    // MeshContent services with its remote mesh router.  Its local pods and
    // VirtualSoulfind HTTP controllers still exist, but remote calls to the
    // corresponding overlay services return the router's not-found contract.
    if target == crate::config::ControllerProfile::Native
        && matches!(service_name, "pods" | "private-gateway" | "shadow-index")
    {
        return false;
    }
    match service_name {
        "private-gateway" | "MeshContent" => features.mesh,
        "pods" => features.pods,
        "shadow-index" => features.virtual_soulfind,
        _ => true,
    }
}

fn local_service_enabled(
    service_name: &str,
    features: &crate::config::FeatureGateSettings,
) -> bool {
    match service_name {
        "private-gateway" | "MeshContent" => features.mesh,
        "pods" => features.pods,
        "shadow-index" => features.virtual_soulfind,
        "dht" => features.dht,
        _ => true,
    }
}

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

    async fn load_or_create_with_quic_and_data_policy_and_proxy_inner(
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
        state: Arc<super::AppState>,
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
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = gateway.handle_connection(tcp, &state).await {
                tracing::debug!(%error, "overlay gateway connection closed");
            }
            gateway.overlay_rate_limiter.record_disconnection(remote_ip);
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
    ) -> String {
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        let timestamp = overlay_timestamp();
        self.overlay_connections.write().await.insert(
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
        connection_id
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
    ) -> OutboundOverlayGuard {
        let connection_id = self
            .register_outbound_overlay(
                username,
                endpoint,
                features,
                version,
                certificate_thumbprint,
            )
            .await;
        OutboundOverlayGuard {
            gateway: Arc::clone(self),
            connection_id,
        }
    }

    pub async fn run(self: Arc<Self>, state: Arc<super::AppState>) -> Result<(), String> {
        let listener = self.listener.lock().await.take();
        if let Some(udp_listener) = self.udp_listener.lock().await.take() {
            let gateway = Arc::clone(&self);
            let udp_state = Arc::clone(&state);
            let quic_proxy_backend = self.quic_proxy_backend;
            let quic_data_proxy_backend = self.quic_data_proxy_backend;
            tokio::spawn(async move {
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
            tokio::spawn(async move {
                gateway.run_quic_control(quic_listener, quic_state).await;
            });
        }
        if let Some(quic_data_listener) = self.quic_data_listener.lock().await.take() {
            let gateway = Arc::clone(&self);
            tokio::spawn(async move {
                gateway.run_quic_data(quic_data_listener).await;
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

    async fn run_udp_control(
        &self,
        socket: UdpSocket,
        state: Arc<super::AppState>,
        quic_proxy_backend: Option<SocketAddr>,
        quic_data_proxy_backend: Option<SocketAddr>,
    ) {
        let public_socket = Arc::new(socket);
        let mut quic_sessions = HashMap::new();
        let quic_admission = QuicProxyAdmissionGate::default();
        if let Some(forward_socket) = self.dht_forward_socket.as_ref() {
            tokio::spawn(forward_dht_responses(
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
                    .store(super::unix_timestamp(), Ordering::Relaxed);
                if session
                    .sender
                    .send(buffer[..received.0].to_vec())
                    .await
                    .is_err()
                {
                    quic_sessions.remove(&received.1);
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
            let Ok(envelope) = ControlEnvelope::decode(&buffer[..received.0]) else {
                continue;
            };
            let now = match i64::try_from(super::unix_timestamp_millis()) {
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

    async fn run_quic_control(
        self: Arc<Self>,
        server: QuicControlServer,
        state: Arc<super::AppState>,
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
            let gateway = Arc::clone(&self);
            let connection_state = Arc::clone(&state);
            tokio::spawn(async move {
                gateway
                    .handle_quic_connection(connection, connection_state)
                    .await;
                gateway.overlay_rate_limiter.record_disconnection(remote_ip);
            });
        }
    }

    async fn run_quic_data(self: Arc<Self>, server: QuicDataServer) {
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
            tokio::spawn(async move {
                let _permit = permit;
                Arc::clone(&gateway)
                    .handle_quic_data_connection(connection)
                    .await;
                gateway.overlay_rate_limiter.record_disconnection(remote_ip);
            });
        }
    }

    async fn handle_quic_data_connection(self: Arc<Self>, connection: QuicDataConnection) {
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

    async fn handle_quic_data_stream(
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

    async fn handle_quic_connection(
        &self,
        connection: QuicControlConnection,
        state: Arc<super::AppState>,
    ) {
        let remote = connection.remote_address();
        loop {
            let envelope =
                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
                    Ok(Ok(envelope)) => envelope,
                    Ok(Err(QuicControlError::Connection(error))) => {
                        tracing::debug!(%error, ?remote, "overlay QUIC control connection closed");
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
            let now = match i64::try_from(super::unix_timestamp_millis()) {
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

struct QuicProxySession {
    sender: mpsc::Sender<Vec<u8>>,
    last_activity: Arc<AtomicU64>,
    address_validated: Arc<AtomicBool>,
    _admission_lease: QuicProxyAdmissionLease,
}

impl QuicProxySession {
    async fn new(
        remote: SocketAddr,
        backend: SocketAddr,
        public_socket: Arc<UdpSocket>,
        admission_lease: QuicProxyAdmissionLease,
    ) -> Result<Self, std::io::Error> {
        let bind = match backend {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let backend_socket = UdpSocket::bind(bind).await?;
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(32);
        let last_activity = Arc::new(AtomicU64::new(super::unix_timestamp()));
        let address_validated = Arc::new(AtomicBool::new(false));
        let task_last_activity = Arc::clone(&last_activity);
        let task_address_validated = Arc::clone(&address_validated);
        tokio::spawn(async move {
            let mut response = vec![0_u8; 65_536];
            loop {
                tokio::select! {
                    packet = receiver.recv() => {
                        let Some(packet) = packet else { return; };
                        if backend_socket.send_to(&packet, backend).await.is_err() {
                            return;
                        }
                        task_last_activity.store(super::unix_timestamp(), Ordering::Relaxed);
                    }
                    received = backend_socket.recv_from(&mut response) => {
                        let Ok((length, source)) = received else { return; };
                        if source != backend {
                            continue;
                        }
                        task_address_validated.store(true, Ordering::Relaxed);
                        task_last_activity.store(super::unix_timestamp(), Ordering::Relaxed);
                        if public_socket.send_to(&response[..length], remote).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });
        Ok(Self {
            sender,
            last_activity,
            address_validated,
            _admission_lease: admission_lease,
        })
    }
}

fn prune_quic_proxy_sessions(sessions: &mut HashMap<SocketAddr, QuicProxySession>) {
    let now = super::unix_timestamp();
    sessions.retain(|_, session| {
        let timeout = if session.address_validated.load(Ordering::Relaxed) {
            QUIC_PROXY_IDLE_TIMEOUT
        } else {
            QUIC_PROXY_PENDING_TIMEOUT
        };
        now.saturating_sub(session.last_activity.load(Ordering::Relaxed)) <= timeout.as_secs()
    });
}

fn is_dht_packet(buffer: &[u8]) -> bool {
    buffer.first().copied() == Some(b'd')
}

/// Return DHT responses from mainline's internal socket through the public
/// shared UDP socket. The source address observed by the backend is the DHT
/// peer that sent the request, so the public socket can send the response to
/// that peer while retaining the configured public source port.
async fn forward_dht_responses(forward_socket: Arc<UdpSocket>, public_socket: Arc<UdpSocket>) {
    let mut buffer = [0_u8; 65_536];
    loop {
        let (received, peer) = match forward_socket.recv_from(&mut buffer).await {
            Ok(received) => received,
            Err(error) => {
                tracing::debug!(%error, "shared DHT response forwarder stopped");
                return;
            }
        };
        if !is_dht_packet(&buffer[..received]) {
            continue;
        }
        if let Err(error) = public_socket.send_to(&buffer[..received], peer).await {
            tracing::debug!(%error, ?peer, "shared DHT response forwarding failed");
        }
    }
}

fn is_quic_initial_packet(buffer: &[u8]) -> bool {
    if buffer.len() < 1_200 || buffer[0] & 0xc0 != 0xc0 {
        return false;
    }
    let version = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
    let packet_type = (buffer[0] & 0x30) >> 4;
    (version == 0x0000_0001 && packet_type == 0) || (version == 0x6b33_43cf && packet_type == 1)
}

fn select_quic_proxy_backend(
    packet: &[u8],
    control_backend: Option<SocketAddr>,
    data_backend: Option<SocketAddr>,
) -> Option<SocketAddr> {
    if let Some(alpn) = quic_alpn::first_alpn(packet) {
        if alpn == "slskdn-overlay-data" {
            return data_backend.or(control_backend);
        }
    }
    control_backend.or(data_backend)
}

#[derive(Clone, Default)]
struct QuicProxyAdmissionGate {
    state: Arc<StdMutex<QuicProxyAdmissionState>>,
}

#[derive(Default)]
struct QuicProxyAdmissionState {
    active_sessions: usize,
    active_by_prefix: HashMap<String, usize>,
    recent_attempts: VecDeque<(u64, String)>,
}

impl QuicProxyAdmissionGate {
    fn try_acquire(&self, remote: SocketAddr) -> Option<QuicProxyAdmissionLease> {
        let prefix = quic_proxy_network_prefix(remote.ip());
        let now = super::unix_timestamp();
        let mut state = self.state.lock().ok()?;
        while state.recent_attempts.front().is_some_and(|(timestamp, _)| {
            now.saturating_sub(*timestamp) > QUIC_PROXY_ATTEMPT_WINDOW.as_secs()
        }) {
            state.recent_attempts.pop_front();
        }
        let active_for_prefix = state.active_by_prefix.get(&prefix).copied().unwrap_or(0);
        let attempts_for_prefix = state
            .recent_attempts
            .iter()
            .filter(|(_, attempted_prefix)| attempted_prefix == &prefix)
            .count();
        if state.active_sessions >= QUIC_PROXY_MAX_SESSIONS
            || active_for_prefix >= QUIC_PROXY_PREFIX_SESSION_LIMIT
            || state.recent_attempts.len() >= QUIC_PROXY_GLOBAL_ATTEMPT_LIMIT
            || attempts_for_prefix >= QUIC_PROXY_PREFIX_ATTEMPT_LIMIT
        {
            return None;
        }
        state.recent_attempts.push_back((now, prefix.clone()));
        state.active_sessions = state.active_sessions.saturating_add(1);
        state
            .active_by_prefix
            .insert(prefix.clone(), active_for_prefix.saturating_add(1));
        Some(QuicProxyAdmissionLease {
            state: Arc::clone(&self.state),
            prefix,
        })
    }

    fn release(&self, prefix: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.active_sessions = state.active_sessions.saturating_sub(1);
        if let Some(active) = state.active_by_prefix.get_mut(prefix) {
            *active = active.saturating_sub(1);
            if *active == 0 {
                state.active_by_prefix.remove(prefix);
            }
        }
    }
}

struct QuicProxyAdmissionLease {
    state: Arc<StdMutex<QuicProxyAdmissionState>>,
    prefix: String,
}

impl Drop for QuicProxyAdmissionLease {
    fn drop(&mut self) {
        let gate = QuicProxyAdmissionGate {
            state: Arc::clone(&self.state),
        };
        gate.release(&self.prefix);
    }
}

fn quic_proxy_network_prefix(address: IpAddr) -> String {
    match address {
        IpAddr::V4(address) => {
            let [first, second, third, _] = address.octets();
            format!("{first}.{second}.{third}")
        }
        IpAddr::V6(address) => match address.to_ipv4() {
            Some(address) => {
                let [first, second, third, _] = address.octets();
                format!("{first}.{second}.{third}")
            }
            None => {
                let bytes = address.octets();
                hex::encode(&bytes[..7])
            }
        },
    }
}

async fn read_quic_data_command_line(
    receive: &mut QuicDataReceiveStream,
) -> Result<(String, Vec<u8>), QuicDataError> {
    let mut bytes = Vec::with_capacity(256);
    let mut byte = [0_u8; 1];
    while bytes.len() < 256 {
        let read = receive.read_chunk(&mut byte).await?;
        if read == 0 {
            break;
        }
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    let line = String::from_utf8_lossy(&bytes).trim_end().to_owned();
    Ok((line, bytes))
}

async fn read_quic_data_command_line_with_timeout(
    receive: &mut QuicDataReceiveStream,
) -> Result<(String, Vec<u8>), QuicDataError> {
    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
        .await
        .map_err(|_| QuicDataError::Timeout("data command read"))?
}

async fn write_quic_data_error(
    send: &mut QuicDataSendStream,
    reason: &str,
) -> Result<(), QuicDataError> {
    timeout(DESTINATION_WRITE_TIMEOUT, async {
        send.write_all(format!("ERR {reason}\n").as_bytes()).await?;
        send.finish()
    })
    .await
    .map_err(|_| QuicDataError::Timeout("data error write"))?
}

fn relay_authentication_valid(line: &str, configured_token: &str) -> bool {
    if configured_token.is_empty() || !line.starts_with("AUTH ") {
        return false;
    }
    let Ok(presented) = BASE64.decode(line[5..].trim()) else {
        return false;
    };
    let expected = configured_token.as_bytes();
    if presented.len() != expected.len() {
        return false;
    }
    let difference = presented
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        });
    difference == 0
}

fn allowed_relay_destination(policy: &QuicDataPolicy, host: &str, port: u16) -> bool {
    let requested = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    policy
        .allowed_relay_destinations
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&requested))
}

async fn copy_quic_to_tcp(
    mut receive: QuicDataReceiveStream,
    mut target: tokio::net::tcp::OwnedWriteHalf,
    max_bytes: u64,
) -> Result<(), String> {
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    while total < max_bytes {
        let remaining =
            usize::try_from((max_bytes - total).min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = receive
            .read_chunk(&mut buffer[..remaining])
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buffer[..read])
            .await
            .map_err(|error| error.to_string())?;
        total = total.saturating_add(read as u64);
    }
    Ok(())
}

async fn copy_tcp_to_quic(
    mut source: tokio::net::tcp::OwnedReadHalf,
    mut send: QuicDataSendStream,
    max_bytes: u64,
) -> Result<(), String> {
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    while total < max_bytes {
        let remaining =
            usize::try_from((max_bytes - total).min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = source
            .read(&mut buffer[..remaining])
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        send.write_all(&buffer[..read])
            .await
            .map_err(|error| error.to_string())?;
        total = total.saturating_add(read as u64);
    }
    send.finish().map_err(|error| error.to_string())
}

impl Gateway {
    async fn handle_connection(
        &self,
        tcp: TcpStream,
        state: &super::AppState,
    ) -> Result<(), String> {
        let remote_address = tcp
            .peer_addr()
            .map_err(|error| format!("overlay peer address failed: {error}"))?;
        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
            .await
            .map_err(|_| "overlay TLS accept timed out".to_owned())?
            .map_err(|error| format!("overlay TLS accept failed: {error}"))?;
        let certificate_thumbprint = tls
            .get_ref()
            .1
            .peer_certificates()
            .and_then(|certificates| certificates.first())
            .map(|certificate| hex::encode(Sha256::digest(certificate.as_ref())));
        let mut framer = OverlayFramer::new(tls);
        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
            .await
            .map_err(|_| "overlay hello timed out".to_owned())?
            .map_err(|error| format!("overlay hello failed: {error}"))?;
        hello
            .validate()
            .map_err(|error| format!("overlay hello rejected: {error}"))?;
        let supports_mesh_service = hello
            .features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SERVICE));
        let supports_mesh_search = hello
            .features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SEARCH));
        if !supports_mesh_service && !supports_mesh_search {
            return Err("overlay peer advertises no supported feature".to_owned());
        }
        authenticate_overlay_peer(state, &hello, remote_address.ip(), &self.certificate_sha256)
            .await?;
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        let local_username = super::pod_request_peer_id(state)
            .await
            .ok_or_else(|| "local gateway identity is unavailable".to_owned())?;
        let features = [
            (FEATURE_MESH_SERVICE, supports_mesh_service),
            (FEATURE_MESH_SEARCH, supports_mesh_search),
        ]
        .into_iter()
        .filter_map(|(feature, supported)| supported.then_some(feature.to_owned()))
        .collect();
        let acknowledgement = MeshHelloAck {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_hello_ack".to_owned(),
            version: OVERLAY_VERSION,
            username: local_username,
            features,
            soulseek_ports: None,
            overlay_port: Some(self.bind().port()),
            nonce_echo: hello.nonce,
        };
        self.overlay_connections.write().await.insert(
            connection_id.clone(),
            OverlayConnectionMetadata {
                username: hello.username.clone(),
                address: remote_address.ip().to_string(),
                port: remote_address.port(),
                features: hello.features.clone(),
                connected_at: overlay_timestamp(),
                last_activity: overlay_timestamp(),
                certificate_thumbprint,
                version: hello.version,
                is_outbound: false,
            },
        );

        let result = async {
            framer
                .write(&acknowledgement)
                .await
                .map_err(|error| format!("overlay acknowledgement failed: {error}"))?;
            let mut liveness = OverlayLiveness::new();
            loop {
                if liveness.is_idle() {
                    return Err("overlay connection was idle too long".to_owned());
                }
                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
                    Ok(result) => {
                        liveness.record_inbound();
                        self.touch_overlay_connection(&connection_id).await;
                        result.map_err(|error| format!("overlay read failed: {error}"))?
                    }
                    Err(_) if liveness.last_ping.elapsed() >= OVERLAY_KEEPALIVE_INTERVAL => {
                        let timestamp = i64::try_from(super::unix_timestamp_millis())
                            .map_err(|_| "overlay clock is out of range".to_owned())?;
                        framer
                            .write(&Ping {
                                magic: OVERLAY_MAGIC.to_owned(),
                                message_type: "ping".to_owned(),
                                version: OVERLAY_VERSION,
                                timestamp,
                            })
                            .await
                            .map_err(|error| format!("overlay keepalive failed: {error}"))?;
                        liveness.record_ping();
                        continue;
                    }
                    Err(_) => continue,
                };
                let message_type = serde_json::from_slice::<serde_json::Value>(&raw)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("type")
                            .and_then(|kind| kind.as_str())
                            .map(str::to_owned)
                    })
                    .ok_or_else(|| "overlay message type is missing".to_owned())?;
                match message_type.as_str() {
                    "mesh_service_call" => {
                        let call: MeshServiceCall = serde_json::from_slice(&raw)
                            .map_err(|error| format!("overlay service call is invalid: {error}"))?;
                        let reply = self
                            .handle_call(call, &hello.username, &connection_id, state)
                            .await;
                        framer
                            .write(&reply)
                            .await
                            .map_err(|error| format!("overlay service reply failed: {error}"))?;
                    }
                    "mesh_search_req" if supports_mesh_search => {
                        let request: MeshSearchRequestMessage = serde_json::from_slice(&raw)
                            .map_err(|error| {
                                format!("overlay mesh search request is invalid: {error}")
                            })?;
                        // The frozen dispatcher drops invalid requests after recording a
                        // violation; it does not manufacture a response for malformed input.
                        if request.validate().is_err() {
                            continue;
                        }
                        let response = self.handle_mesh_search(request, state).await;
                        framer.write(&response).await.map_err(|error| {
                            format!("overlay mesh search response failed: {error}")
                        })?;
                    }
                    "mesh_search_req" => {
                        return Err("overlay mesh search is not negotiated".to_owned());
                    }
                    "ping" => {
                        let ping: Ping = serde_json::from_slice(&raw)
                            .map_err(|error| format!("overlay ping is invalid: {error}"))?;
                        ping.validate()
                            .map_err(|_| "overlay ping is invalid".to_owned())?;
                        framer
                            .write(&Pong {
                                magic: OVERLAY_MAGIC.to_owned(),
                                message_type: "pong".to_owned(),
                                version: OVERLAY_VERSION,
                                timestamp: ping.timestamp,
                            })
                            .await
                            .map_err(|error| format!("overlay pong failed: {error}"))?;
                    }
                    "pong" => {
                        let pong: Pong = serde_json::from_slice(&raw)
                            .map_err(|error| format!("overlay pong is invalid: {error}"))?;
                        pong.validate()
                            .map_err(|_| "overlay pong is invalid".to_owned())?;
                    }
                    "disconnect" => return Ok(()),
                    _ => return Err("unsupported overlay message type".to_owned()),
                }
            }
        }
        .await;
        self.remove_connection_tunnels(&connection_id).await;
        self.overlay_connections
            .write()
            .await
            .remove(&connection_id);
        result
    }

    async fn handle_mesh_search(
        &self,
        request: MeshSearchRequestMessage,
        state: &super::AppState,
    ) -> MeshSearchResponseMessage {
        let request_id = request.request_id.clone();
        let search = timeout(Duration::from_secs(5), async {
            let entries = state.shares.read().await.entries.clone();
            let mut matches = super::search_shares(&entries, &request.search_text);
            matches.sort_by(|left, right| left.filename.cmp(&right.filename));

            let max_results = usize::try_from(request.max_results).unwrap_or(1);
            let truncated = matches.len() > max_results;
            matches.truncate(max_results);
            let files = matches
                .iter()
                .filter_map(mesh_search_file_dto)
                .collect::<Vec<_>>();
            MeshSearchResponseMessage::new(request.request_id, files, truncated, None)
        })
        .await;

        match search {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                tracing::debug!(%error, "mesh search response validation failed");
                mesh_search_error_response(request_id, "Search failed")
            }
            Err(_) => mesh_search_error_response(request_id, "Search failed"),
        }
    }

    async fn handle_call(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &super::AppState,
    ) -> MeshServiceReply {
        self.handle_call_with_mode(call, remote_username, connection_id, state, false)
            .await
    }

    async fn handle_call_with_mode(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &super::AppState,
        local_http: bool,
    ) -> MeshServiceReply {
        let service_enabled = {
            let media_services = state.media_services.read().await;
            if local_http {
                local_service_enabled(call.service_name.as_str(), &media_services.features)
            } else {
                overlay_service_enabled(
                    call.service_name.as_str(),
                    &media_services.features,
                    state.config.controller_profile,
                )
            }
        };
        let result = if !service_enabled {
            Err((2, format!("Service '{}' not found", call.service_name)))
        } else if call.magic != OVERLAY_MAGIC
            || call.message_type != "mesh_service_call"
            || call.version != OVERLAY_VERSION
            || call.correlation_id.trim().is_empty()
            || call.payload.len() > MAX_OVERLAY_MESSAGE_BYTES
        {
            Err((4, "Invalid service call".to_owned()))
        } else {
            match call.service_name.as_str() {
                "private-gateway" => match call.method.as_str() {
                    "OpenTunnel" => {
                        self.open_tunnel(&call.payload, remote_username, connection_id, state)
                            .await
                    }
                    "TunnelData" => {
                        self.tunnel_data(&call.payload, remote_username, connection_id)
                            .await
                    }
                    "GetTunnelData" => {
                        self.get_tunnel_data(&call.payload, remote_username, connection_id)
                            .await
                    }
                    "CloseTunnel" => {
                        self.close_tunnel(&call.payload, remote_username, connection_id)
                            .await
                    }
                    _ => Err((3, "Unknown method".to_owned())),
                },
                "pods" => {
                    self.handle_pods_call(&call.method, &call.payload, remote_username, state)
                        .await
                }
                "shadow-index" => {
                    self.handle_shadow_index_call(&call.method, &call.payload, state)
                        .await
                }
                "MeshContent" => {
                    self.handle_mesh_content_call(&call.method, &call.payload, state)
                        .await
                }
                "dht" => {
                    self.dht_service
                        .handle_call(&call.method, &call.payload, remote_username)
                        .await
                }
                _ => Err((2, "Unknown service".to_owned())),
            }
        };
        match result {
            Ok(payload) => service_reply(call.correlation_id, 0, payload, None),
            Err((status, error)) => {
                service_reply(call.correlation_id, status, Vec::new(), Some(error))
            }
        }
    }

    /// Dispatch an HTTP-gateway service call through the same real service
    /// handlers used by authenticated overlay peers.  This keeps the local
    /// gateway from inventing a compatibility record when it is configured as
    /// a provider for the HTTP gateway.
    pub async fn call_http_service(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &super::AppState,
    ) -> MeshServiceReply {
        self.handle_call_with_mode(call, remote_username, connection_id, state, true)
            .await
    }

    async fn handle_shadow_index_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        match method {
            "QueryByMbid" => {
                let request: ShadowQueryRequest = parse_payload(payload)?;
                let mbid = valid_shadow_mbid(&request.mbid)?;
                let result = shadow_index_result(state, mbid)
                    .await
                    .ok_or_else(|| (2, "No data found for MBID".to_owned()))?;
                serde_json::to_vec(&result)
                    .map_err(|_| (1, "Shadow-index response failed".to_owned()))
            }
            "QueryBatch" => {
                let request: ShadowBatchRequest = parse_payload(payload)?;
                if request.mbids.is_empty() || request.mbids.len() > MAX_SHADOW_BATCH {
                    return Err((
                        if request.mbids.len() > MAX_SHADOW_BATCH {
                            9
                        } else {
                            4
                        },
                        "MBIDs list is invalid".to_owned(),
                    ));
                }
                let mut results = serde_json::Map::new();
                let mut seen = std::collections::HashSet::new();
                for mbid in request.mbids {
                    let mbid = valid_shadow_mbid(&mbid)?;
                    if !seen.insert(mbid.to_owned()) {
                        continue;
                    }
                    if let Some(result) = shadow_index_result(state, mbid).await {
                        results.insert(mbid.to_owned(), result);
                    }
                }
                serde_json::to_vec(&results)
                    .map_err(|_| (1, "Shadow-index response failed".to_owned()))
            }
            _ => Err((3, "Unknown method".to_owned())),
        }
    }

    async fn handle_mesh_content_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        if method != "GetByContentId" {
            return Err((3, "Unknown method".to_owned()));
        }
        let request: MeshContentRequest = parse_payload(payload)?;
        let content_id = bounded_required(&request.content_id, MAX_CONTENT_ID_BYTES, "ContentId")?;
        let (local_path, indexed_size) = {
            let shares = state.shares.read().await;
            let entry = shares
                .entries
                .iter()
                .find(|entry| {
                    entry.filename == content_id
                        || super::stable_content_hash(&entry.filename, entry.size).to_string()
                            == content_id
                })
                .ok_or_else(|| (2, "Content not found or not advertisable".to_owned()))?;
            let local_path = shares
                .local_paths
                .get(&entry.filename)
                .cloned()
                .ok_or_else(|| (2, "Content not found or not advertisable".to_owned()))?;
            (local_path, entry.size)
        };
        let mut file = super::open_shared_local_file(state, &local_path)
            .await
            .map_err(|_| (2, "Content not found or not advertisable".to_owned()))?;
        let actual_size = file
            .metadata()
            .map_err(|_| (10, "Content metadata failed".to_owned()))?
            .len();
        if actual_size != indexed_size || actual_size == 0 {
            return Err((2, "Content not found or not advertisable".to_owned()));
        }
        let (offset, length) = mesh_content_range(request.range.as_ref(), actual_size)?;
        let bytes = tokio::task::spawn_blocking(move || {
            file.seek(SeekFrom::Start(offset))?;
            let mut bytes = vec![0_u8; length];
            file.read_exact(&mut bytes)?;
            Ok::<_, std::io::Error>(bytes)
        })
        .await
        .map_err(|_| (10, "Content read task failed".to_owned()))?
        .map_err(|_| (10, "Content read failed".to_owned()))?;
        Ok(bytes)
    }

    async fn handle_pods_call(
        &self,
        method: &str,
        payload: &[u8],
        remote_username: &str,
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        match method {
            "List" => serde_json::to_vec(&state.pods.read().await.list_visible(None))
                .map_err(|_| (1, "Pod response failed".to_owned())),
            "Get" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let pods = state.pods.read().await;
                let pod = pods
                    .get(pod_id)
                    .filter(|_| pods.is_public(pod_id) || pods.is_member(pod_id, remote_username))
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&pod).map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "Join" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let joined = state
                    .pods
                    .write()
                    .await
                    .join(pod_id, remote_username.to_owned())
                    .map_err(|error| (8, error))?
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&serde_json::json!({"Success": joined}))
                    .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "Leave" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let left = state
                    .pods
                    .write()
                    .await
                    .leave(pod_id, remote_username)
                    .map_err(|error| (8, error))?
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&serde_json::json!({"Success": left}))
                    .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "PostMessage" => {
                let request: PodMessageRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let channel_id =
                    bounded_required(&request.channel_id, MAX_POD_ID_BYTES, "ChannelId")?;
                if request.body.trim().is_empty() || request.body.len() > MAX_POD_MESSAGE_BODY_BYTES
                {
                    return Err((9, "Message body is invalid".to_owned()));
                }
                let binding = {
                    let pods = state.pods.read().await;
                    if !pods.channel_exists(pod_id, channel_id) {
                        return Err((2, "Pod channel not found".to_owned()));
                    }
                    if !pods.is_member(pod_id, remote_username) {
                        return Err((8, "Pod membership is required".to_owned()));
                    }
                    pods.soulseek_binding(pod_id, channel_id)
                };
                let message = state
                    .pod_channels
                    .write()
                    .await
                    .append(
                        pod_id.to_owned(),
                        channel_id.to_owned(),
                        remote_username.to_owned(),
                        request.body,
                        request.signature.unwrap_or_default(),
                        super::unix_timestamp_millis(),
                    )
                    .map_err(|error| (1, error))?;
                if let Some(binding) =
                    binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
                {
                    let room = binding.identifier;
                    if let Err(error) = super::try_send_session_command(
                        state,
                        super::SessionCommand::SayRoom {
                            room: room.clone(),
                            body: format!("[Pod:{}] {}", message.sender_peer_id, message.body),
                        },
                    ) {
                        super::record_pod_room_mirror_failure(state, &room, &error).await;
                    }
                }
                serde_json::to_vec(&serde_json::json!({
                    "Success": true,
                    "MessageId": message.message_id,
                }))
                .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "GetMessages" => {
                let request: PodMessagesRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let channel_id =
                    bounded_required(&request.channel_id, MAX_POD_ID_BYTES, "ChannelId")?;
                let pods = state.pods.read().await;
                if !pods.channel_exists(pod_id, channel_id) {
                    return Err((2, "Pod channel not found".to_owned()));
                }
                if !pods.is_member(pod_id, remote_username) {
                    return Err((8, "Pod membership is required".to_owned()));
                }
                drop(pods);
                let since = match request.since_timestamp {
                    Some(value) => Some(
                        u64::try_from(value)
                            .map_err(|_| (4, "SinceTimestamp is invalid".to_owned()))?,
                    ),
                    None => None,
                };
                let messages = state
                    .pod_channels
                    .read()
                    .await
                    .list(pod_id, channel_id, since);
                serde_json::to_vec(&messages).map_err(|_| (1, "Pod response failed".to_owned()))
            }
            _ => Err((3, "Unknown method".to_owned())),
        }
    }

    async fn open_tunnel(
        &self,
        payload: &[u8],
        remote_username: &str,
        connection_id: &str,
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: OpenTunnelRequest = parse_payload(payload)?;
        let now = super::unix_timestamp();
        if !valid_open_tunnel_request(&request)
            || request.request_timestamp < 0
            || now.abs_diff(request.request_timestamp as u64) > REQUEST_FRESHNESS_SECONDS
        {
            return Err((4, "Invalid tunnel request".to_owned()));
        }
        let local_username = super::pod_request_peer_id(state)
            .await
            .ok_or_else(|| (10, "Gateway identity is unavailable".to_owned()))?;
        {
            let pods = state.pods.read().await;
            let pod = pods
                .get(&request.pod_id)
                .ok_or_else(|| (2, "Pod not found".to_owned()))?;
            if !pods.is_member(&request.pod_id, remote_username) {
                return Err((8, "Only pod members can open tunnels".to_owned()));
            }
            let gateway = pod
                .private_service_policy
                .as_ref()
                .and_then(|policy| policy.get("gatewayPeerId"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if gateway != local_username {
                return Err((10, "Request reached a non-gateway peer".to_owned()));
            }
            if !pods.destination_allowed(
                &request.pod_id,
                &request.destination_host,
                request.destination_port,
            ) {
                return Err((8, "Destination is not allowed by pod policy".to_owned()));
            }
        }
        {
            let mut nonces = self.replay_nonces.lock().await;
            nonces.retain(|_, seen| now.saturating_sub(*seen) <= REQUEST_FRESHNESS_SECONDS);
            let key = (remote_username.to_owned(), request.request_nonce.clone());
            if nonces.contains_key(&key) {
                return Err((8, "Tunnel request nonce was replayed".to_owned()));
            }
            if nonces
                .keys()
                .filter(|(username, _)| username.eq_ignore_ascii_case(remote_username))
                .count()
                >= MAX_REPLAY_NONCES_PER_PEER
            {
                return Err((
                    6,
                    "Tunnel request replay quota is full for this peer".to_owned(),
                ));
            }
            if nonces.len() >= MAX_REPLAY_NONCES {
                return Err((6, "Tunnel request replay cache is full".to_owned()));
            }
            nonces.insert(key, now);
        }
        let tunnels = self.tunnels.read().await;
        if tunnels.len() >= MAX_TUNNELS
            || tunnels
                .values()
                .filter(|tunnel| tunnel.owner == remote_username)
                .count()
                >= MAX_TUNNELS_PER_PEER
        {
            return Err((6, "Tunnel capacity is full".to_owned()));
        }
        drop(tunnels);
        let destination = resolve_destination(&request.destination_host, request.destination_port)
            .await
            .map_err(|error| (10, error))?;
        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
            .await
            .map_err(|_| (10, "Destination connection timed out".to_owned()))?
            .map_err(|_| (10, "Destination connection failed".to_owned()))?;
        let (mut reader, writer) = stream.into_split();
        let (incoming_tx, incoming_rx) = mpsc::channel(INBOUND_BUFFER_CHUNKS);
        let tunnel_id = uuid::Uuid::new_v4().simple().to_string();
        let mut tunnels = self.tunnels.write().await;
        if tunnels.len() >= MAX_TUNNELS
            || tunnels
                .values()
                .filter(|tunnel| tunnel.owner == remote_username)
                .count()
                >= MAX_TUNNELS_PER_PEER
        {
            return Err((6, "Tunnel capacity is full".to_owned()));
        }
        tunnels.insert(
            tunnel_id.clone(),
            Arc::new(Tunnel {
                owner: remote_username.to_owned(),
                connection_id: connection_id.to_owned(),
                pod_id: request.pod_id,
                writer: Mutex::new(writer),
                incoming: Mutex::new(incoming_rx),
            }),
        );
        drop(tunnels);
        tokio::spawn(async move {
            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
            while let Ok(read) = reader.read(&mut buffer).await {
                if read == 0 || incoming_tx.send(buffer[..read].to_vec()).await.is_err() {
                    break;
                }
            }
        });
        serde_json::to_vec(&OpenTunnelResponse {
            tunnel_id,
            accepted: true,
        })
        .map_err(|_| (1, "Tunnel response failed".to_owned()))
    }

    async fn tunnel_data(
        &self,
        payload: &[u8],
        remote_username: &str,
        connection_id: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: TunnelDataRequest = parse_payload(payload)?;
        if request.data.len() > TUNNEL_CHUNK_BYTES {
            return Err((9, "Tunnel payload is too large".to_owned()));
        }
        let tunnel = self
            .owned_tunnel(&request.tunnel_id, remote_username, connection_id)
            .await?;
        let mut writer = tunnel.writer.lock().await;
        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))
            .await
            .map_err(|_| (10, "Tunnel write timed out".to_owned()))?
            .map_err(|_| (10, "Tunnel write failed".to_owned()))?;
        serde_json::to_vec(&serde_json::json!({"Sent": request.data.len()}))
            .map_err(|_| (1, "Tunnel response failed".to_owned()))
    }

    async fn get_tunnel_data(
        &self,
        payload: &[u8],
        remote_username: &str,
        connection_id: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: GetTunnelDataRequest = parse_payload(payload)?;
        let tunnel = self
            .owned_tunnel(&request.tunnel_id, remote_username, connection_id)
            .await?;
        let data = match tunnel.incoming.lock().await.try_recv() {
            Ok(data) => data,
            Err(mpsc::error::TryRecvError::Empty) => Vec::new(),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.tunnels.write().await.remove(&request.tunnel_id);
                return Err((10, "Destination closed the tunnel".to_owned()));
            }
        };
        serde_json::to_vec(&TunnelDataResponse {
            bytes_received: data.len(),
            data,
        })
        .map_err(|_| (1, "Tunnel response failed".to_owned()))
    }

    async fn close_tunnel(
        &self,
        payload: &[u8],
        remote_username: &str,
        connection_id: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: CloseTunnelRequest = parse_payload(payload)?;
        self.owned_tunnel(&request.tunnel_id, remote_username, connection_id)
            .await?;
        self.tunnels.write().await.remove(&request.tunnel_id);
        Ok(br#"{"Closed":true}"#.to_vec())
    }

    async fn owned_tunnel(
        &self,
        tunnel_id: &str,
        remote_username: &str,
        connection_id: &str,
    ) -> Result<Arc<Tunnel>, (i32, String)> {
        let tunnel = self
            .tunnels
            .read()
            .await
            .get(tunnel_id)
            .cloned()
            .ok_or_else(|| (2, "Tunnel not found".to_owned()))?;
        if tunnel.owner != remote_username || tunnel.connection_id != connection_id {
            return Err((8, "Tunnel belongs to another peer".to_owned()));
        }
        Ok(tunnel)
    }

    async fn remove_connection_tunnels(&self, connection_id: &str) {
        self.tunnels
            .write()
            .await
            .retain(|_, tunnel| tunnel.connection_id != connection_id);
    }

    async fn touch_overlay_connection(&self, connection_id: &str) {
        if let Some(connection) = self
            .overlay_connections
            .write()
            .await
            .get_mut(connection_id)
        {
            connection.last_activity = overlay_timestamp();
        }
    }
}

#[derive(Debug)]
struct Tunnel {
    owner: String,
    connection_id: String,
    #[allow(
        dead_code,
        reason = "retained for tunnel audit and future quota projection"
    )]
    pod_id: String,
    writer: Mutex<OwnedWriteHalf>,
    incoming: Mutex<mpsc::Receiver<Vec<u8>>>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayConnectionMetadata {
    pub username: String,
    pub address: String,
    pub port: u16,
    pub features: Vec<String>,
    pub connected_at: String,
    pub last_activity: String,
    pub certificate_thumbprint: Option<String>,
    pub version: i32,
    pub is_outbound: bool,
}

pub struct OutboundOverlayGuard {
    gateway: Arc<Gateway>,
    connection_id: String,
}

impl Drop for OutboundOverlayGuard {
    fn drop(&mut self) {
        let gateway = Arc::clone(&self.gateway);
        let connection_id = self.connection_id.clone();
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                gateway.remove_overlay_connection(&connection_id).await;
            });
        }
    }
}

fn overlay_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn mesh_search_file_dto(entry: &super::FileEntry) -> Option<MeshSearchFileDto> {
    let size = i64::try_from(entry.size).ok()?;
    let extension = (!entry.extension.is_empty()).then(|| entry.extension.clone());
    let bitrate = entry
        .attributes
        .iter()
        .find(|attribute| attribute.code == 0)
        .and_then(|attribute| i32::try_from(attribute.value).ok());
    let duration = entry
        .attributes
        .iter()
        .find(|attribute| attribute.code == 1)
        .and_then(|attribute| i32::try_from(attribute.value).ok());

    Some(MeshSearchFileDto {
        filename: entry.filename.clone(),
        size,
        extension,
        bitrate,
        duration,
        codec: mesh_search_codec(entry.extension.as_str()),
        media_kinds: mesh_search_media_kinds(entry.extension.as_str()),
        content_id: None,
        hash: None,
    })
}

fn mesh_search_error_response(request_id: String, error: &str) -> MeshSearchResponseMessage {
    MeshSearchResponseMessage::new(request_id, Vec::new(), false, Some(error.to_owned()))
        .expect("validated mesh search request id must produce a valid error response")
}

fn mesh_search_codec(extension: &str) -> Option<String> {
    match extension
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "flac" => Some("FLAC".to_owned()),
        "mp3" => Some("MP3".to_owned()),
        "m4a" | "aac" => Some("AAC".to_owned()),
        "opus" => Some("Opus".to_owned()),
        "ogg" => Some("Vorbis".to_owned()),
        "wav" => Some("WAV".to_owned()),
        _ => None,
    }
}

fn mesh_search_media_kinds(extension: &str) -> Option<Vec<String>> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    let mut kinds = Vec::new();
    if matches!(
        extension.as_str(),
        "mp3" | "flac" | "m4a" | "aac" | "opus" | "ogg" | "wav" | "wma" | "ape" | "mka"
    ) {
        kinds.push("Music".to_owned());
    }
    if matches!(
        extension.as_str(),
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
    ) {
        kinds.push("Video".to_owned());
    }
    if matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico"
    ) {
        kinds.push("Image".to_owned());
    }
    (!kinds.is_empty()).then_some(kinds)
}

fn service_reply(
    correlation_id: String,
    status_code: i32,
    payload: Vec<u8>,
    error_message: Option<String>,
) -> MeshServiceReply {
    MeshServiceReply {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "mesh_service_reply".to_owned(),
        version: OVERLAY_VERSION,
        correlation_id,
        status_code,
        payload,
        error_message,
    }
}

fn parse_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, (i32, String)> {
    serde_json::from_slice(payload).map_err(|_| (4, "Invalid request payload".to_owned()))
}

#[derive(Debug, serde::Deserialize)]
struct ShadowQueryRequest {
    #[serde(alias = "MBID", alias = "mbid")]
    mbid: String,
}

#[derive(Debug, serde::Deserialize)]
struct ShadowBatchRequest {
    #[serde(alias = "MBIDs", alias = "mbids")]
    mbids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MeshContentRequest {
    #[serde(alias = "ContentId", alias = "contentId")]
    content_id: String,
    #[serde(default, alias = "Range", alias = "range")]
    range: Option<MeshContentRange>,
}

#[derive(Debug, serde::Deserialize)]
struct MeshContentRange {
    #[serde(alias = "Offset", alias = "offset")]
    offset: i64,
    #[serde(alias = "Length", alias = "length")]
    length: i64,
}

fn valid_shadow_mbid(value: &str) -> Result<&str, (i32, String)> {
    let value = value.trim();
    if !(8..=MAX_SHADOW_MBID_BYTES).contains(&value.len())
        || value.contains("..")
        || value.contains(['/', '\\'])
        || value.chars().any(char::is_control)
    {
        return Err((4, "Invalid MBID".to_owned()));
    }
    Ok(value)
}

async fn shadow_index_result(state: &super::AppState, mbid: &str) -> Option<serde_json::Value> {
    let discovery = state.content_discovery.read().await;
    let shadow = discovery
        .shadow_records()
        .iter()
        .find(|record| record.recording_id.eq_ignore_ascii_case(mbid))?;
    let canonical_variants = discovery
        .hash_entries()
        .iter()
        .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(mbid))
        .take(10)
        .filter_map(|entry| {
            let hash = [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|hash| !hash.is_empty())?;
            let hash = hex::decode(hash).ok()?;
            Some(serde_json::json!({
                "Codec": "FLAC",
                "BitrateKbps": 0,
                "SizeBytes": entry.size,
                "HashPrefix": BASE64.encode(&hash[..hash.len().min(16)]),
                "QualityScore": 1.0,
            }))
        })
        .collect::<Vec<_>>();
    let last_updated = chrono::DateTime::from_timestamp(shadow.updated_at as i64, 0)
        .map(|timestamp| timestamp.to_rfc3339());
    Some(serde_json::json!({
        "MBID": shadow.recording_id,
        "PeerCount": shadow.peer_ids.len(),
        "CanonicalVariants": canonical_variants,
        "LastUpdated": last_updated,
    }))
}

fn mesh_content_range(
    requested: Option<&MeshContentRange>,
    size: u64,
) -> Result<(u64, usize), (i32, String)> {
    let (offset, requested_length) = match requested {
        Some(range) if range.offset >= 0 && range.length >= 0 => {
            (range.offset as u64, range.length as u64)
        }
        Some(_) => return Err((4, "Invalid range request".to_owned())),
        None => (0, size),
    };
    if offset >= size {
        return Err((4, "Invalid range request".to_owned()));
    }
    let remaining = size - offset;
    let length = if requested_length == 0 {
        remaining
    } else {
        requested_length.min(remaining)
    };
    if length == 0 {
        return Err((4, "Invalid range request".to_owned()));
    }
    if length > MAX_MESH_CONTENT_BYTES as u64 {
        return Err((9, "Range too large; request a smaller range".to_owned()));
    }
    Ok((offset, length as usize))
}

#[derive(Debug, serde::Deserialize)]
struct PodIdRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct PodMessageRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
    #[serde(alias = "ChannelId")]
    channel_id: String,
    #[serde(alias = "Body")]
    body: String,
    #[serde(default, alias = "Signature")]
    signature: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PodControlMessage {
    #[serde(rename = "MessageId", alias = "messageId")]
    message_id: String,
    #[serde(rename = "SenderPeerId", alias = "senderPeerId")]
    sender_peer_id: String,
    #[serde(rename = "TimestampUnixMs", alias = "timestampUnixMs")]
    timestamp_unix_ms: i64,
}

#[derive(Debug, serde::Deserialize)]
struct PodMessagesRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
    #[serde(alias = "ChannelId")]
    channel_id: String,
    #[serde(default, alias = "SinceTimestamp")]
    since_timestamp: Option<i64>,
}

fn bounded_required<'a>(
    value: &'a str,
    maximum: usize,
    name: &str,
) -> Result<&'a str, (i32, String)> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum {
        return Err((4, format!("{name} is invalid")));
    }
    Ok(value)
}

fn valid_open_tunnel_request(request: &OpenTunnelRequest) -> bool {
    !request.pod_id.trim().is_empty()
        && request.pod_id.len() <= MAX_POD_ID_BYTES
        && !request.destination_host.trim().is_empty()
        && request.destination_host.len() <= MAX_DESTINATION_HOST_BYTES
        && request.destination_port != 0
        && request.service_name.as_ref().is_none_or(|service| {
            !service.trim().is_empty() && service.len() <= MAX_SERVICE_NAME_BYTES
        })
        && !request.request_nonce.trim().is_empty()
        && request.request_nonce.len() <= MAX_REQUEST_NONCE_BYTES
}

async fn resolve_destination(host: &str, port: u16) -> Result<SocketAddr, String> {
    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
        .await
        .map_err(|_| "Destination resolution timed out".to_owned())?
        .map_err(|_| "Destination resolution failed".to_owned())?;
    addresses
        .find(|address| valid_destination_ip(address.ip()))
        .ok_or_else(|| "Destination did not resolve to a usable address".to_owned())
}

async fn resolve_public_relay_destination(host: &str, port: u16) -> Result<SocketAddr, String> {
    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
        .await
        .map_err(|_| "Relay destination resolution timed out".to_owned())?
        .map_err(|_| "Relay destination resolution failed".to_owned())?;
    addresses
        .find(|address| valid_public_relay_ip(address.ip()))
        .ok_or_else(|| "Relay destination is not public".to_owned())
}

async fn authenticate_overlay_peer(
    state: &super::AppState,
    hello: &MeshHello,
    remote_ip: IpAddr,
    gateway_certificate_sha256: &[u8; 32],
) -> Result<(), String> {
    if state
        .security
        .read()
        .await
        .is_blocked("ip", &remote_ip.to_string())
        || state
            .security
            .read()
            .await
            .is_blocked("username", &hello.username)
    {
        return Err("overlay peer is blocklisted".to_owned());
    }
    let public_key = state
        .mesh
        .read()
        .await
        .capability_records
        .iter()
        .find(|record| {
            record.username.eq_ignore_ascii_case(&hello.username)
                && record.expires_at_unix > super::unix_timestamp()
        })
        .map(|record| record.public_key)
        .ok_or_else(|| "overlay peer has no fresh authenticated capability record".to_owned())?;
    verify_overlay_peer_authentication(hello, &public_key, gateway_certificate_sha256)?;
    let expected = super::request_peer_endpoint(state, &hello.username)
        .await
        .map_err(|_| "overlay peer Soulseek endpoint is unavailable".to_owned())?;
    if remote_ip != IpAddr::V4(expected.ip) {
        return Err("overlay peer IP does not match its Soulseek endpoint".to_owned());
    }
    Ok(())
}

fn verify_overlay_peer_authentication(
    hello: &MeshHello,
    expected_public_key: &[u8; 32],
    gateway_certificate_sha256: &[u8; 32],
) -> Result<(), String> {
    hello
        .verify_authentication(expected_public_key, gateway_certificate_sha256)
        .map_err(|_| "overlay peer failed capability-key authentication".to_owned())
}

fn valid_destination_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && !address.is_broadcast()
                && (address.is_private() || address.is_loopback() || address.is_link_local())
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && (address.is_unique_local()
                    || address.is_loopback()
                    || address.is_unicast_link_local())
        }
    }
}

fn valid_public_relay_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => {
            let [first, second, third, ..] = address.octets();
            first != 0
                && first != 10
                && first != 127
                && !(first == 100 && (64..=127).contains(&second))
                && !(first == 169 && second == 254)
                && !(first == 172 && (16..=31).contains(&second))
                && !(first == 192
                    && ((second == 0 && (third == 0 || third == 2))
                        || (second == 31 && third == 196)
                        || (second == 52 && third == 193)
                        || (second == 88 && third == 99)))
                && !(first == 192 && second == 168)
                && !(first == 198 && (second == 18 || second == 19))
                && !(first == 198 && second == 51 && third == 100)
                && !(first == 203 && second == 0 && third == 113)
                && first < 224
        }
        IpAddr::V6(address) => {
            let bytes = address.octets();
            if let Some(mapped) = address.to_ipv4() {
                return valid_public_relay_ip(IpAddr::V4(mapped));
            }
            let segments = address.segments();
            let special_use = segments[0] == 0x2001
                && (segments[1] == 0x0002
                    || matches!(segments[1] & 0xfff0, 0x0010 | 0x0020)
                    || segments[1] == 0x0db8)
                || (segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
                || (segments[0] == 0x0100
                    && segments[1] == 0
                    && segments[2] == 0
                    && segments[3] == 0);
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_unicast_link_local()
                && !address.is_unique_local()
                && !address.is_multicast()
                && !special_use
                && (bytes[0] & 0xfe) != 0xfc
        }
    }
}

fn load_or_create_certificate(
    state_dir: &Path,
) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), String> {
    let certificate_path = state_dir.join("overlay-certificate.der");
    let private_key_path = state_dir.join("overlay-private-key.der");
    let certificate = read_identity_file(
        &certificate_path,
        "certificate",
        MAX_CERTIFICATE_BYTES,
        false,
    )?;
    let private_key = read_identity_file(
        &private_key_path,
        "private key",
        MAX_PRIVATE_KEY_BYTES,
        true,
    )?;
    match (certificate, private_key) {
        (Some(certificate), Some(private_key)) => {
            return Ok((
                CertificateDer::from(certificate),
                PrivatePkcs8KeyDer::from(private_key),
            ));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(
                "overlay TLS identity is incomplete; both certificate and private key are required"
                    .to_owned(),
            );
        }
        (None, None) => {}
    }
    fs::create_dir_all(state_dir)
        .map_err(|error| format!("overlay state directory creation failed: {error}"))?;
    let certified = generate_simple_self_signed(vec!["localhost".to_owned()])
        .map_err(|error| format!("overlay certificate generation failed: {error}"))?;
    let certificate = certified.cert.der().to_vec();
    let private_key = certified.signing_key.serialize_der();
    write_new_identity(
        &certificate_path,
        &private_key_path,
        &certificate,
        &private_key,
    )?;
    Ok((
        CertificateDer::from(certificate),
        PrivatePkcs8KeyDer::from(private_key),
    ))
}

fn write_new_identity(
    certificate_path: &Path,
    private_key_path: &Path,
    certificate: &[u8],
    private_key: &[u8],
) -> Result<(), String> {
    write_secret(certificate_path, certificate)?;
    if let Err(error) = write_secret(private_key_path, private_key) {
        return match fs::remove_file(certificate_path) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; overlay certificate rollback failed: {cleanup_error}"
            )),
        };
    }
    Ok(())
}

fn read_identity_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
    require_private_permissions: bool,
) -> Result<Option<Vec<u8>>, String> {
    #[cfg(not(unix))]
    let _ = require_private_permissions;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("overlay {label} metadata failed: {error}")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| format!("overlay {label} metadata failed: {error}"))?;
    if !opened_metadata.is_file() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    #[cfg(unix)]
    if require_private_permissions {
        use std::os::unix::fs::PermissionsExt;
        if opened_metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "overlay {label} must not be accessible by group or other users"
            ));
        }
    }
    if opened_metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut bytes = Vec::new();
    std::io::Read::take(&mut file, max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    if bytes.is_empty() {
        return Err(format!("overlay {label} is empty"));
    }
    Ok(Some(bytes))
}

fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_CLOEXEC);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("overlay identity creation failed: {error}"))?;
    if let Err(error) = std::io::Write::write_all(&mut file, bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity write failed: {error}"));
    }
    drop(file);
    if let Err(error) = fs::hard_link(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity publish failed: {error}"));
    }
    if let Err(error) = fs::remove_file(&temporary) {
        return Err(format!(
            "overlay identity published but temporary cleanup failed: {error}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "slskr-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn gateway_destinations_are_confined_to_private_networks() {
        assert!(valid_destination_ip("127.0.0.1".parse().unwrap()));
        assert!(valid_destination_ip("10.0.0.1".parse().unwrap()));
        assert!(valid_destination_ip("fd00::1".parse().unwrap()));
        assert!(!valid_destination_ip("8.8.8.8".parse().unwrap()));
        assert!(!valid_destination_ip(
            "2001:4860:4860::8888".parse().unwrap()
        ));
        assert!(!valid_destination_ip("0.0.0.0".parse().unwrap()));
    }

    #[test]
    fn quic_relay_destinations_are_confined_to_public_addresses() {
        assert!(valid_public_relay_ip("8.8.8.8".parse().unwrap()));
        assert!(valid_public_relay_ip(
            "2001:4860:4860::8888".parse().unwrap()
        ));
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.1.1",
            "224.0.0.1",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(
                !valid_public_relay_ip(address.parse().unwrap()),
                "{address}"
            );
        }
        for address in [
            "192.0.0.1",
            "192.0.2.1",
            "192.31.196.1",
            "192.52.193.1",
            "192.88.99.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "::ffff:10.0.0.1",
            "::ffff:192.0.2.1",
            "2001:db8::1",
            "2001:2::1",
            "2001:20::1",
            "100::1",
        ] {
            assert!(
                !valid_public_relay_ip(address.parse().unwrap()),
                "{address}"
            );
        }
    }

    #[test]
    fn quic_relay_authentication_uses_the_configured_token_bytes() {
        let encoded = BASE64.encode("relay-token");
        assert!(relay_authentication_valid(
            &format!("AUTH {encoded}"),
            "relay-token"
        ));
        assert!(!relay_authentication_valid(
            &format!("AUTH {encoded}"),
            "other-token"
        ));
        assert!(!relay_authentication_valid(
            "AUTH not-base64",
            "relay-token"
        ));
    }

    #[test]
    fn overlay_services_follow_selected_profile_feature_gates() {
        let mut features = crate::config::FeatureGateSettings::default();
        assert!(!overlay_service_enabled(
            "private-gateway",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(!overlay_service_enabled(
            "MeshContent",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(!overlay_service_enabled(
            "pods",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(!overlay_service_enabled(
            "shadow-index",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(overlay_service_enabled(
            "dht",
            &features,
            crate::config::ControllerProfile::Legacy
        ));

        features.mesh = true;
        features.pods = true;
        features.virtual_soulfind = true;
        assert!(overlay_service_enabled(
            "private-gateway",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(overlay_service_enabled(
            "MeshContent",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(overlay_service_enabled(
            "pods",
            &features,
            crate::config::ControllerProfile::Legacy
        ));
        assert!(overlay_service_enabled(
            "shadow-index",
            &features,
            crate::config::ControllerProfile::Legacy
        ));

        assert!(!overlay_service_enabled(
            "private-gateway",
            &features,
            crate::config::ControllerProfile::Native
        ));
        assert!(!overlay_service_enabled(
            "pods",
            &features,
            crate::config::ControllerProfile::Native
        ));
        assert!(!overlay_service_enabled(
            "shadow-index",
            &features,
            crate::config::ControllerProfile::Native
        ));
        assert!(overlay_service_enabled(
            "MeshContent",
            &features,
            crate::config::ControllerProfile::Native
        ));
    }

    #[test]
    fn quic_proxy_admission_matches_frozen_global_and_prefix_limits() {
        let gate = QuicProxyAdmissionGate::default();
        let first_prefix = [
            "198.51.100.1:50305",
            "198.51.100.2:50305",
            "198.51.100.3:50305",
            "198.51.100.4:50305",
        ];
        let leases = first_prefix
            .into_iter()
            .map(|address| gate.try_acquire(address.parse().unwrap()))
            .collect::<Option<Vec<_>>>();
        assert!(leases.is_some());
        assert!(gate
            .try_acquire("198.51.100.5:50305".parse().unwrap())
            .is_none());
        drop(leases);

        let global_gate = QuicProxyAdmissionGate::default();
        let mut global_leases = Vec::new();
        for third_octet in 0..64_u8 {
            let address = SocketAddr::from(([203, 0, third_octet, 1], 50_305));
            global_leases.push(
                global_gate
                    .try_acquire(address)
                    .expect("global admission slot"),
            );
        }
        assert!(global_gate
            .try_acquire("203.0.64.1:50305".parse().unwrap())
            .is_none());
    }

    #[test]
    fn quic_packet_classifier_accepts_only_frozen_initial_shapes() {
        let mut packet = vec![0_u8; 1_200];
        packet[0] = 0xc0;
        packet[1..5].copy_from_slice(&1_u32.to_be_bytes());
        assert!(is_quic_initial_packet(&packet));
        packet[1..5].copy_from_slice(&0x6b33_43cf_u32.to_be_bytes());
        packet[0] = 0xd0;
        assert!(is_quic_initial_packet(&packet));
        packet[0] = 0x40;
        assert!(!is_quic_initial_packet(&packet));
        assert!(is_dht_packet(b"d1:ad2:id20:01234567890123456789ee"));
        assert!(!is_dht_packet(b"\xc0"));
    }

    #[tokio::test]
    async fn shared_dht_response_forwarder_returns_packets_from_public_source() {
        let public_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let forward_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let backend_peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let public_address = public_socket.local_addr().unwrap();
        let forward_address = forward_socket.local_addr().unwrap();
        let forwarder = tokio::spawn(forward_dht_responses(
            Arc::clone(&forward_socket),
            Arc::clone(&public_socket),
        ));

        let packet = b"d1:ad2:id20:01234567890123456789ee";
        backend_peer.send_to(packet, forward_address).await.unwrap();
        let mut received = [0_u8; 128];
        let (size, source) = tokio::time::timeout(
            Duration::from_secs(1),
            backend_peer.recv_from(&mut received),
        )
        .await
        .expect("DHT response should be forwarded")
        .unwrap();
        assert_eq!(&received[..size], packet);
        assert_eq!(source, public_address);

        forwarder.abort();
    }

    #[test]
    fn overlay_keepalive_and_control_validation_match_the_frozen_lifecycle() {
        assert_eq!(OVERLAY_MESSAGE_READ_TIMEOUT, Duration::from_secs(30));
        assert_eq!(OVERLAY_KEEPALIVE_INTERVAL, Duration::from_secs(120));
        assert_eq!(OVERLAY_IDLE_TIMEOUT, Duration::from_secs(300));

        let now = i64::try_from(super::super::unix_timestamp_millis()).unwrap();
        assert!(Ping {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "ping".to_owned(),
            version: OVERLAY_VERSION,
            timestamp: now,
        }
        .validate()
        .is_ok());

        let mut liveness = OverlayLiveness {
            last_inbound: Instant::now() - OVERLAY_IDLE_TIMEOUT,
            last_ping: Instant::now() - OVERLAY_KEEPALIVE_INTERVAL,
        };
        assert!(liveness.is_idle());
        liveness.record_ping();
        assert!(liveness.is_idle(), "outbound pings are not peer activity");
        liveness.record_inbound();
        assert!(!liveness.is_idle());
    }

    #[test]
    fn gateway_tunnel_request_fields_are_bounded_before_replay_caching() {
        let request = OpenTunnelRequest {
            pod_id: "pod".to_owned(),
            destination_host: "service.local".to_owned(),
            destination_port: 80,
            service_name: None,
            request_nonce: "n".repeat(MAX_REQUEST_NONCE_BYTES),
            request_timestamp: 1,
        };
        assert!(valid_open_tunnel_request(&request));

        let mut oversized = request.clone();
        oversized.request_nonce.push('n');
        assert!(!valid_open_tunnel_request(&oversized));
        oversized = request.clone();
        oversized.destination_host = "h".repeat(MAX_DESTINATION_HOST_BYTES + 1);
        assert!(!valid_open_tunnel_request(&oversized));
        oversized = request;
        oversized.service_name = Some(String::new());
        assert!(!valid_open_tunnel_request(&oversized));
    }

    #[test]
    fn gateway_requires_capability_key_authentication() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[17; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let certificate_sha256 = [23; 32];
        let mut hello = MeshHello::new(
            "peer",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            Some(443),
            "nonce",
        )
        .unwrap();

        assert!(
            verify_overlay_peer_authentication(&hello, &public_key, &certificate_sha256).is_err()
        );

        hello
            .authenticate(&signing_key, &certificate_sha256)
            .unwrap();
        assert!(
            verify_overlay_peer_authentication(&hello, &public_key, &certificate_sha256).is_ok()
        );
    }

    #[tokio::test]
    async fn gateway_certificate_identity_is_durable() {
        let root = temporary_directory("gateway-identity");
        let first = Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
            .await
            .unwrap();
        let second = Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
            .await
            .unwrap();
        assert_eq!(first.certificate_sha256(), second.certificate_sha256());
        drop((first, second));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn shared_tcp_gateway_does_not_bind_a_second_public_tcp_socket() {
        let root = temporary_directory("gateway-shared-tcp");
        let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let public_address = public_listener.local_addr().unwrap();
        let gateway = Gateway::load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share_shared_tcp(
            public_address,
            &root,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            8,
            false,
        )
        .await
        .unwrap();

        assert_eq!(gateway.bind(), public_address);
        assert!(gateway.listener.lock().await.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn outbound_overlay_metadata_is_removed_when_guard_drops() {
        let root = temporary_directory("gateway-outbound-session");
        let gateway = Arc::new(
            Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
                .await
                .unwrap(),
        );
        let guard = gateway
            .register_outbound_guard(
                "remote".to_owned(),
                "192.0.2.10:2234".parse().unwrap(),
                vec![FEATURE_MESH_SERVICE.to_owned()],
                OVERLAY_VERSION,
                None,
            )
            .await;
        let connections = gateway.active_overlay_connections().await;
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].username, "remote");
        assert_eq!(connections[0].address, "192.0.2.10");
        assert_eq!(connections[0].port, 2234);
        assert!(connections[0].is_outbound);
        drop(guard);
        tokio::task::yield_now().await;
        assert!(gateway.active_overlay_connections().await.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incomplete_gateway_identity_is_rejected() {
        let root = temporary_directory("gateway-incomplete-identity");
        fs::write(root.join("overlay-certificate.der"), [1_u8]).unwrap();
        let error = load_or_create_certificate(&root).unwrap_err();
        assert!(error.contains("identity is incomplete"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_gateway_identity_is_rejected_before_parsing() {
        let root = temporary_directory("gateway-oversized-identity");
        fs::write(
            root.join("overlay-certificate.der"),
            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
        )
        .unwrap();
        fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
        let error = load_or_create_certificate(&root).unwrap_err();
        assert!(error.contains("certificate is too large"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_gateway_identity_is_rejected() {
        use std::os::unix::fs::symlink;

        let root = temporary_directory("gateway-symlinked-identity");
        let certificate_target = root.join("certificate-target.der");
        fs::write(&certificate_target, [1_u8]).unwrap();
        symlink(&certificate_target, root.join("overlay-certificate.der")).unwrap();
        fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
        let error = load_or_create_certificate(&root).unwrap_err();
        assert!(error.contains("certificate must be a regular file"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn exposed_gateway_private_key_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_directory("gateway-exposed-private-key");
        let path = root.join("overlay-private-key.der");
        fs::write(&path, [1_u8]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        let error = read_identity_file(&path, "private key", MAX_PRIVATE_KEY_BYTES, true)
            .expect_err("reject exposed private key");
        assert!(error.contains("must not be accessible by group or other users"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_identity_publish_removes_temporary_secret() {
        let root = temporary_directory("gateway-failed-secret-publish");
        let destination = root.join("overlay-private-key.der");
        fs::create_dir(&destination).unwrap();

        let error = write_secret(&destination, b"private-key").unwrap_err();
        assert!(error.contains("publish failed"));
        let names = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(names, vec![destination.file_name().unwrap()]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_private_key_publish_rolls_back_new_certificate() {
        let root = temporary_directory("gateway-identity-rollback");
        let certificate = root.join("overlay-certificate.der");
        let private_key = root.join("overlay-private-key.der");
        fs::create_dir(&private_key).unwrap();

        let error = write_new_identity(&certificate, &private_key, b"certificate", b"private-key")
            .unwrap_err();
        assert!(error.contains("publish failed"));
        assert!(!certificate.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn secret_publish_never_replaces_existing_identity() {
        let root = temporary_directory("gateway-existing-identity");
        let path = root.join("overlay-private-key.der");
        fs::write(&path, b"existing-key").unwrap();

        let error = write_secret(&path, b"replacement-key").unwrap_err();

        assert!(error.contains("publish failed"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), b"existing-key");
        let temporary_files = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "overlay-private-key.der")
            .count();
        assert_eq!(temporary_files, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
