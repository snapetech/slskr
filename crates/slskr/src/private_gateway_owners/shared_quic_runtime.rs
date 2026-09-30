use super::*;
use slskr_client::shared_udp::{SharedQuicIngress, SharedUdpSocket};
use std::sync::Weak;

pub(super) struct SharedQuicTransport {
    pub(super) socket: SharedUdpSocket,
    pub(super) ingress: Option<SharedQuicIngress>,
    pub(super) validated: StdMutex<HashMap<SocketAddr, Weak<AtomicBool>>>,
}

struct SharedSession {
    ingress: SharedQuicIngress,
    last_activity: u64,
    validated: Arc<AtomicBool>,
    _lease: quic_proxy::QuicProxyAdmissionLease,
}

impl SharedQuicTransport {
    pub(super) fn mark_validated(&self, remote: SocketAddr) {
        if let Ok(peers) = self.validated.lock() {
            if let Some(flag) = peers.get(&remote).and_then(Weak::upgrade) {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    fn prune(&self, sessions: &mut HashMap<SocketAddr, SharedSession>, now: u64) {
        sessions.retain(|_, session| {
            let idle = if session.validated.load(Ordering::Relaxed) {
                QUIC_PROXY_IDLE_TIMEOUT
            } else {
                QUIC_PROXY_PENDING_TIMEOUT
            };
            now.saturating_sub(session.last_activity) <= idle.as_secs()
        });
        if let Ok(mut peers) = self.validated.lock() {
            peers.retain(|remote, flag| sessions.contains_key(remote) && flag.strong_count() > 0);
        }
    }
}

impl Gateway {
    pub(super) async fn run_shared_quic(
        self: Arc<Self>,
        server: slskr_client::shared_quic_server::SharedQuicServer,
        state: Arc<crate::AppState>,
    ) {
        while let Some(connection) = server.accept().await {
            let connection = match connection {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::debug!(%error, "shared QUIC connection rejected");
                    continue;
                }
            };
            let remote = connection.remote_address();
            if let Some(shared) = self.shared_quic.as_ref() {
                shared.mark_validated(remote);
            }
            if !self
                .overlay_rate_limiter
                .check_connection(remote.ip())
                .allowed
            {
                continue;
            }
            let Ok(permit) = Arc::clone(&self.connections).try_acquire_owned() else {
                self.overlay_rate_limiter.record_disconnection(remote.ip());
                continue;
            };
            let admission = GatewayConnectionAdmission {
                limiter: Arc::clone(&self.overlay_rate_limiter),
                remote_ip: remote.ip(),
            };
            let gateway = Arc::clone(&self);
            let connection_state = Arc::clone(&state);
            state.managed_background_tasks.spawn(async move {
                let _permit = permit;
                let _admission = admission;
                match connection {
                    slskr_client::shared_quic_server::SharedQuicConnection::Control(connection) => {
                        gateway
                            .handle_quic_connection(connection, connection_state)
                            .await;
                    }
                    slskr_client::shared_quic_server::SharedQuicConnection::Data(connection) => {
                        gateway.handle_quic_data_connection(connection).await;
                    }
                }
            });
        }
    }

    pub(super) async fn run_shared_udp_control(
        &self,
        shared: &SharedQuicTransport,
        state: Arc<crate::AppState>,
    ) {
        let mut sessions: HashMap<SocketAddr, SharedSession> = HashMap::new();
        let admission = QuicProxyAdmissionGate::default();
        let mut buffer = vec![0_u8; 128 * 1024];
        let mut sweep = tokio::time::interval(Duration::from_secs(1));
        sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let meta = tokio::select! {
                _ = sweep.tick() => {
                    shared.prune(&mut sessions, crate::unix_timestamp());
                    continue;
                }
                received = shared.socket.recv(&mut buffer) => match received {
                    Ok(meta) => meta,
                    Err(error) => {
                        tracing::debug!(%error, "shared UDP listener stopped");
                        return;
                    }
                }
            };
            if meta.stride == 0 || meta.len > buffer.len() {
                continue;
            }
            // Quinn's native socket may return a GRO batch. Preserve each
            // datagram boundary and its kernel-observed metadata before routing.
            for packet in buffer[..meta.len].chunks(meta.stride) {
                if shared_dht_packet(packet) {
                    if let Some(dht) = state.dht.as_ref() {
                        dht.accept_shared_udp_datagram(packet, meta.addr);
                    }
                    continue;
                }
                if shared_quic_session_packet(packet) {
                    let mut single = meta;
                    single.len = packet.len();
                    single.stride = packet.len();
                    if let Some(session) = sessions.get_mut(&meta.addr) {
                        match session.ingress.try_send(packet, single) {
                            Ok(()) => session.last_activity = crate::unix_timestamp(),
                            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                                sessions.remove(&meta.addr);
                                if let Ok(mut peers) = shared.validated.lock() {
                                    peers.remove(&meta.addr);
                                }
                            }
                            Err(_) => {}
                        }
                        continue;
                    }
                    if !is_quic_initial_packet(packet) || sessions.len() >= QUIC_PROXY_MAX_SESSIONS
                    {
                        continue;
                    }
                    // Bound new peer attempts before handing Initials to Quinn.
                    let Some(lease) = admission.try_acquire(meta.addr) else {
                        continue;
                    };
                    let Some(ingress) = shared.ingress.clone() else {
                        continue;
                    };
                    let validated = Arc::new(AtomicBool::new(false));
                    let Ok(mut peers) = shared.validated.lock() else {
                        continue;
                    };
                    if peers.len() >= QUIC_PROXY_MAX_SESSIONS {
                        continue;
                    }
                    // Register before the endpoint can complete a handshake.
                    peers.insert(meta.addr, Arc::downgrade(&validated));
                    if ingress.try_send(packet, single).is_err() {
                        peers.remove(&meta.addr);
                        continue;
                    }
                    sessions.insert(
                        meta.addr,
                        SharedSession {
                            ingress,
                            last_activity: crate::unix_timestamp(),
                            validated,
                            _lease: lease,
                        },
                    );
                    continue;
                }
                self.handle_udp_overlay_datagram(packet, meta.addr, &state)
                    .await;
            }
        }
    }
}

// After negotiation QUIC may grease its fixed bit. Short headers still have
// a clear header-form bit; long headers carry the visible protocol version.
fn shared_quic_session_packet(packet: &[u8]) -> bool {
    packet.first().is_some_and(|byte| byte & 0x80 == 0)
        || packet
            .get(1..5)
            .is_some_and(|version| version == [0, 0, 0, 1] || version == [0x6b, 0x33, 0x43, 0xcf])
}

fn shared_dht_packet(packet: &[u8]) -> bool {
    #[cfg(slskr_mainline_outbound_socket)]
    {
        mainline::is_dht_datagram(packet)
    }
    #[cfg(not(slskr_mainline_outbound_socket))]
    {
        let _ = packet;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_quic_short_header_is_not_dht_just_because_it_starts_with_d() {
        assert!(!shared_dht_packet(&[0x64; 1200]));
        assert!(!shared_dht_packet(b"dictionary-shaped garbage"));
        assert!(shared_dht_packet(
            b"d1:ad2:id20:AAAAAAAAAAAAAAAAAAAAe1:q4:ping1:t4:aaaa1:y1:qe"
        ));
    }
    #[tokio::test]
    async fn shared_idle_sweep_releases_pending_leases_and_preserves_validated_peers() {
        let public = StdUdpSocket::bind("127.0.0.1:0").unwrap();
        let socket = SharedUdpSocket::new(&public).unwrap();
        let (ingress, _endpoint) = socket.endpoint();
        let shared = SharedQuicTransport {
            socket,
            ingress: Some(ingress.clone()),
            validated: StdMutex::new(HashMap::new()),
        };
        let gate = QuicProxyAdmissionGate::default();
        let mut sessions = HashMap::new();
        for (port, valid) in [(1234, false), (1235, true)] {
            let remote = SocketAddr::from(([127, 0, 0, 1], port));
            let flag = Arc::new(AtomicBool::new(false));
            shared
                .validated
                .lock()
                .unwrap()
                .insert(remote, Arc::downgrade(&flag));
            sessions.insert(
                remote,
                SharedSession {
                    ingress: ingress.clone(),
                    last_activity: 100,
                    validated: flag,
                    _lease: gate.try_acquire(remote).unwrap(),
                },
            );
            if valid {
                shared.mark_validated(remote);
            }
        }
        shared.prune(&mut sessions, 111);
        assert_eq!(sessions.len(), 1);
        assert_eq!(shared.validated.lock().unwrap().len(), 1);
        assert_eq!(gate.state.lock().unwrap().active_sessions, 1);
        shared.prune(&mut sessions, 221);
        assert!(sessions.is_empty());
        assert!(shared.validated.lock().unwrap().is_empty());
        assert_eq!(gate.state.lock().unwrap().active_sessions, 0);
    }
    #[test]
    fn negotiated_quic_fixed_bit_greasing_keeps_short_and_long_session_packets() {
        assert!(shared_quic_session_packet(&[0x04, 1, 2]));
        assert!(shared_quic_session_packet(&[0x64, 1, 2]));
        assert!(shared_quic_session_packet(&[0x80, 0, 0, 0, 1]));
        assert!(!shared_quic_session_packet(&[0x99, 1, 2]));
    }
}
