use super::*;

impl Gateway {
    pub(super) async fn run_udp_control(
        &self,
        socket: UdpSocket,
        state: Arc<crate::AppState>,
        quic_proxy_backend: Option<SocketAddr>,
        quic_data_proxy_backend: Option<SocketAddr>,
    ) {
        if let Some(shared) = self.shared_quic.as_ref() {
            self.run_shared_udp_control(shared, state).await;
            return;
        }
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
                if state.dht.as_ref().is_some_and(|rendezvous| {
                    rendezvous.accept_shared_udp_datagram(&buffer[..received.0], received.1)
                }) {
                    continue;
                }
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
                    &state.managed_background_tasks,
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
            self.handle_udp_overlay_datagram(&buffer[..received.0], received.1, &state)
                .await;
        }
    }

    pub(super) async fn handle_udp_overlay_datagram(
        &self,
        packet: &[u8],
        remote: SocketAddr,
        state: &crate::AppState,
    ) {
        if !self
            .overlay_rate_limiter
            .check_message(&overlay_datagram_limiter_id(remote))
            .allowed
        {
            return;
        }
        let Ok(envelope) = ControlEnvelope::decode(packet) else {
            return;
        };
        let now = match i64::try_from(crate::unix_timestamp_millis()) {
            Ok(now) => now,
            Err(_) => return,
        };
        if !envelope.timestamp_is_current(now) || envelope.verify().is_err() {
            return;
        }
        if envelope.message_type != "pod_message" {
            // Target ControlDispatcher intentionally ignores unknown
            // control types after decode; retain that one-way behavior.
            return;
        }
        let Ok(message) = serde_json::from_slice::<PodControlMessage>(&envelope.payload) else {
            return;
        };
        if message.sender_peer_id.trim().is_empty()
            || message.message_id.trim().is_empty()
            || message.timestamp_unix_ms <= 0
        {
            return;
        }
        if let Err((status, error)) = self
            .handle_pods_call(
                "PostMessage",
                &envelope.payload,
                message.sender_peer_id.trim(),
                state,
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
