use super::*;

pub(super) struct OverlayLiveness {
    pub(super) last_inbound: Instant,
    pub(super) last_ping: Instant,
}

impl OverlayLiveness {
    pub(super) fn new() -> Self {
        let now = Instant::now();
        Self {
            last_inbound: now,
            last_ping: now,
        }
    }

    pub(super) fn record_inbound(&mut self) {
        self.last_inbound = Instant::now();
    }

    pub(super) fn record_ping(&mut self) {
        self.last_ping = Instant::now();
    }

    pub(super) fn is_idle(&self) -> bool {
        self.last_inbound.elapsed() >= OVERLAY_IDLE_TIMEOUT
    }

    pub(super) fn read_wait(&self) -> Duration {
        OVERLAY_MESSAGE_READ_TIMEOUT.min(
            OVERLAY_KEEPALIVE_INTERVAL
                .checked_sub(self.last_ping.elapsed())
                .unwrap_or(Duration::ZERO),
        )
    }
}

#[derive(Clone, Debug)]
pub struct QuicDataPolicy {
    pub relay_authentication_token: String,
    pub allowed_relay_destinations: Vec<String>,
    pub max_concurrent_relays: usize,
    pub max_relay_bytes_per_direction: u64,
    pub max_relay_duration: Duration,
}

pub struct Gateway {
    pub(super) bind: StdRwLock<SocketAddr>,
    pub(super) acceptor: TlsAcceptor,
    pub(super) certificate_sha256: [u8; 32],
    pub(super) listener: Mutex<Option<TcpListener>>,
    pub(super) udp_listener: Mutex<Option<UdpSocket>>,
    pub(super) dht_forward_socket: Option<Arc<UdpSocket>>,
    pub(super) dht_forward_target: Option<SocketAddr>,
    pub(super) quic_listener: Mutex<Option<QuicControlServer>>,
    pub(super) quic_data_listener: Mutex<Option<QuicDataServer>>,
    pub(super) shared_quic: Option<SharedQuicTransport>,
    pub(super) quic_proxy_backend: Option<SocketAddr>,
    pub(super) quic_data_proxy_backend: Option<SocketAddr>,
    pub(super) quic_data_policy: Option<Arc<QuicDataPolicy>>,
    pub(super) quic_data_max_concurrent_streams: usize,
    pub(super) quic_data_relays: Arc<Semaphore>,
    pub(super) connections: Arc<Semaphore>,
    pub(super) tunnels: RwLock<BTreeMap<String, Arc<Tunnel>>>,
    pub(super) overlay_connections: StdRwLock<BTreeMap<String, OverlayConnectionMetadata>>,
    pub(super) replay_nonces: Mutex<BTreeMap<(String, String), u64>>,
    pub(super) overlay_rate_limiter: Arc<OverlayRateLimiter>,
    pub(super) dht_service: crate::mesh_dht::DhtServiceState,
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

#[derive(Debug)]
pub(super) struct Tunnel {
    pub(super) owner: String,
    pub(super) connection_id: String,
    #[allow(
        dead_code,
        reason = "retained for tunnel audit and future quota projection"
    )]
    pub(super) pod_id: String,
    pub(super) writer: Mutex<OwnedWriteHalf>,
    pub(super) incoming: Mutex<mpsc::Receiver<Vec<u8>>>,
    pub(super) reader_abort: AbortHandle,
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.reader_abort.abort();
    }
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
    pub(super) gateway: Arc<Gateway>,
    pub(super) connection_id: String,
}

impl Drop for OutboundOverlayGuard {
    fn drop(&mut self) {
        self.gateway.remove_overlay_metadata(&self.connection_id);
    }
}

/// Releases admission even if a handler is cancelled or never polled.
pub(super) struct GatewayConnectionAdmission {
    pub(super) limiter: Arc<OverlayRateLimiter>,
    pub(super) remote_ip: IpAddr,
}

impl Drop for GatewayConnectionAdmission {
    fn drop(&mut self) {
        self.limiter.record_disconnection(self.remote_ip);
    }
}

/// Metadata access never holds this synchronous lock across an await.
pub(super) struct OverlayMetadataGuard<'a> {
    pub(super) gateway: &'a Gateway,
    pub(super) connection_id: String,
}

impl Drop for OverlayMetadataGuard<'_> {
    fn drop(&mut self) {
        self.gateway.remove_overlay_metadata(&self.connection_id);
        self.gateway
            .overlay_rate_limiter
            .remove_connection(&self.connection_id);
    }
}
