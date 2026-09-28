use super::*;

pub(super) struct QuicProxySession {
    pub(super) sender: mpsc::Sender<Vec<u8>>,
    pub(super) last_activity: Arc<AtomicU64>,
    pub(super) address_validated: Arc<AtomicBool>,
    pub(super) _admission_lease: QuicProxyAdmissionLease,
    pub(super) worker_abort: AbortHandle,
}

impl QuicProxySession {
    pub(super) async fn new(
        remote: SocketAddr,
        backend: SocketAddr,
        public_socket: Arc<UdpSocket>,
        admission_lease: QuicProxyAdmissionLease,
        tasks: &crate::managed_tasks::ManagedTaskRegistry,
    ) -> Result<Self, std::io::Error> {
        let bind = match backend {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let backend_socket = UdpSocket::bind(bind).await?;
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(32);
        let last_activity = Arc::new(AtomicU64::new(crate::unix_timestamp()));
        let address_validated = Arc::new(AtomicBool::new(false));
        let task_last_activity = Arc::clone(&last_activity);
        let task_address_validated = Arc::clone(&address_validated);
        let worker_abort = tasks
            .try_spawn_with_abort(async move {
                let mut response = vec![0_u8; 65_536];
                loop {
                    tokio::select! {
                        packet = receiver.recv() => {
                            let Some(packet) = packet else { return; };
                            if backend_socket.send_to(&packet, backend).await.is_err() {
                                return;
                            }
                            task_last_activity.store(crate::unix_timestamp(), Ordering::Relaxed);
                        }
                        received = backend_socket.recv_from(&mut response) => {
                            let Ok((length, source)) = received else { return; };
                            if source != backend {
                                continue;
                            }
                            task_address_validated.store(true, Ordering::Relaxed);
                            task_last_activity.store(crate::unix_timestamp(), Ordering::Relaxed);
                            if public_socket.send_to(&response[..length], remote).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            })
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Interrupted, "daemon is shutting down")
            })?;
        Ok(Self {
            sender,
            last_activity,
            address_validated,
            _admission_lease: admission_lease,
            worker_abort,
        })
    }
}

impl Drop for QuicProxySession {
    fn drop(&mut self) {
        self.worker_abort.abort();
    }
}

pub(super) fn prune_quic_proxy_sessions(sessions: &mut HashMap<SocketAddr, QuicProxySession>) {
    let now = crate::unix_timestamp();
    sessions.retain(|_, session| {
        let timeout = if session.address_validated.load(Ordering::Relaxed) {
            QUIC_PROXY_IDLE_TIMEOUT
        } else {
            QUIC_PROXY_PENDING_TIMEOUT
        };
        now.saturating_sub(session.last_activity.load(Ordering::Relaxed)) <= timeout.as_secs()
    });
}

pub(super) fn is_dht_packet(buffer: &[u8]) -> bool {
    buffer.first().copied() == Some(b'd')
}

pub(super) fn overlay_datagram_limiter_id(remote: SocketAddr) -> String {
    remote.ip().to_string()
}

/// Return DHT responses from mainline's internal socket through the public
/// shared UDP socket. The source address observed by the backend is the DHT
/// peer that sent the request, so the public socket can send the response to
/// that peer while retaining the configured public source port.
pub(super) async fn forward_dht_responses(
    forward_socket: Arc<UdpSocket>,
    public_socket: Arc<UdpSocket>,
) {
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

pub(super) fn is_quic_initial_packet(buffer: &[u8]) -> bool {
    if buffer.len() < 1_200 || buffer[0] & 0xc0 != 0xc0 {
        return false;
    }
    let version = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
    let packet_type = (buffer[0] & 0x30) >> 4;
    (version == 0x0000_0001 && packet_type == 0) || (version == 0x6b33_43cf && packet_type == 1)
}

pub(super) fn select_quic_proxy_backend(
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
pub(super) struct QuicProxyAdmissionGate {
    pub(super) state: Arc<StdMutex<QuicProxyAdmissionState>>,
}

#[derive(Default)]
pub(super) struct QuicProxyAdmissionState {
    pub(super) active_sessions: usize,
    pub(super) active_by_prefix: HashMap<String, usize>,
    pub(super) recent_attempts: VecDeque<(u64, String)>,
}

impl QuicProxyAdmissionGate {
    pub(super) fn try_acquire(&self, remote: SocketAddr) -> Option<QuicProxyAdmissionLease> {
        let prefix = quic_proxy_network_prefix(remote.ip());
        let now = crate::unix_timestamp();
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

    pub(super) fn release(&self, prefix: &str) {
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

pub(super) struct QuicProxyAdmissionLease {
    pub(super) state: Arc<StdMutex<QuicProxyAdmissionState>>,
    pub(super) prefix: String,
}

impl Drop for QuicProxyAdmissionLease {
    fn drop(&mut self) {
        let gate = QuicProxyAdmissionGate {
            state: Arc::clone(&self.state),
        };
        gate.release(&self.prefix);
    }
}

pub(super) fn quic_proxy_network_prefix(address: IpAddr) -> String {
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
