//! Bounded QUIC data-plane transport for the slskdN overlay.
//!
//! The frozen runtime uses a separate `slskdn-overlay-data` ALPN for bulk
//! payload streams.  Each payload occupies one bidirectional QUIC stream and
//! is terminated by the sender's FIN.  This module deliberately keeps the
//! stream boundary and payload cap explicit so a peer cannot turn a data
//! connection into an unbounded allocation.

use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use quinn::crypto::rustls::QuicClientConfig;
use tokio::{sync::Mutex, time::timeout};
use tokio_rustls::rustls::{
    self,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::WebPkiSupportedAlgorithms,
    pki_types::{CertificateDer, ServerName, UnixTime},
    server::ParsedCertificate,
    DigitallySignedStruct, RootCertStore, SignatureScheme,
};

use crate::quic_control::certificate_public_key_pin;

pub const QUIC_DATA_ALPN: &[u8] = b"slskdn-overlay-data";
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 512 * 1024;
pub const DEFAULT_MAX_CONCURRENT_STREAMS: u32 = 8;
pub const DEFAULT_MAX_CACHED_CONNECTIONS: usize = 64;
const QUIC_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUIC_SEND_GRACE: Duration = Duration::from_millis(100);

/// A bounded QUIC data-plane listener using the frozen slskdN ALPN.
pub struct QuicDataServer {
    endpoint: quinn::Endpoint,
    max_payload_bytes: usize,
}

impl QuicDataServer {
    /// Bind a QUIC data listener with the supplied self-signed identity.
    pub fn bind(
        bind: SocketAddr,
        certificate: rustls::pki_types::CertificateDer<'static>,
        private_key: rustls::pki_types::PrivatePkcs8KeyDer<'static>,
        max_payload_bytes: usize,
    ) -> Result<Self, QuicDataError> {
        Self::bind_with_limits(
            bind,
            certificate,
            private_key,
            max_payload_bytes,
            DEFAULT_MAX_CONCURRENT_STREAMS,
        )
    }

    /// Bind a listener with explicit payload and inbound-stream bounds.
    pub fn bind_with_limits(
        bind: SocketAddr,
        certificate: rustls::pki_types::CertificateDer<'static>,
        private_key: rustls::pki_types::PrivatePkcs8KeyDer<'static>,
        max_payload_bytes: usize,
        max_concurrent_streams: u32,
    ) -> Result<Self, QuicDataError> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate],
                rustls::pki_types::PrivateKeyDer::Pkcs8(private_key),
            )
            .map_err(|error| QuicDataError::Transport(error.to_string()))?;
        server_crypto.alpn_protocols = vec![QUIC_DATA_ALPN.to_vec()];
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .map_err(|error| QuicDataError::Transport(error.to_string()))?,
        ));
        let max_concurrent_streams = max_concurrent_streams.clamp(1, 1_024);
        let mut transport_config = quinn::TransportConfig::default();
        transport_config
            .max_concurrent_bidi_streams(quinn::VarInt::from_u32(max_concurrent_streams));
        transport_config
            .max_concurrent_uni_streams(quinn::VarInt::from_u32(max_concurrent_streams));
        let mut server_config = server_config;
        server_config.transport_config(Arc::new(transport_config));
        let endpoint = quinn::Endpoint::server(server_config, bind)
            .map_err(|error| QuicDataError::Transport(error.to_string()))?;
        Ok(Self {
            endpoint,
            max_payload_bytes: max_payload_bytes.max(1),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, QuicDataError> {
        self.endpoint
            .local_addr()
            .map_err(|error| QuicDataError::Transport(error.to_string()))
    }

    /// Accept the next data connection. `None` means the endpoint was closed.
    pub async fn accept(&self) -> Option<Result<QuicDataConnection, QuicDataError>> {
        let incoming = self.endpoint.accept().await?;
        Some(
            incoming
                .await
                .map(|connection| QuicDataConnection {
                    connection,
                    max_payload_bytes: self.max_payload_bytes,
                })
                .map_err(|error| QuicDataError::Connection(error.to_string())),
        )
    }

    pub fn close(&self) {
        self.endpoint.close(0_u32.into(), b"data listener closed");
    }
}

/// One accepted QUIC data connection.
pub struct QuicDataConnection {
    connection: quinn::Connection,
    max_payload_bytes: usize,
}

impl QuicDataConnection {
    #[must_use]
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Accept one bidirectional payload stream.
    pub async fn accept_stream(&self) -> Result<QuicDataStream, QuicDataError> {
        let (send, receive) = self
            .connection
            .accept_bi()
            .await
            .map_err(|error| QuicDataError::Connection(error.to_string()))?;
        Ok(QuicDataStream {
            send,
            receive,
            max_payload_bytes: self.max_payload_bytes,
        })
    }

    /// Accept either inbound QUIC stream type under the configured payload cap.
    pub async fn accept_inbound_stream(&self) -> Result<QuicDataInboundStream, QuicDataError> {
        tokio::select! {
            result = self.connection.accept_bi() => {
                let (send, receive) = result
                    .map_err(|error| QuicDataError::Connection(error.to_string()))?;
                Ok(QuicDataInboundStream::Bidirectional(QuicDataStream {
                    send,
                    receive,
                    max_payload_bytes: self.max_payload_bytes,
                }))
            }
            result = self.connection.accept_uni() => {
                let receive = result
                    .map_err(|error| QuicDataError::Connection(error.to_string()))?;
                Ok(QuicDataInboundStream::Unidirectional(QuicDataReceiveStream {
                    receive,
                    max_payload_bytes: self.max_payload_bytes,
                }))
            }
        }
    }
}

/// An inbound QUIC data stream accepted by the server.
pub enum QuicDataInboundStream {
    Bidirectional(QuicDataStream),
    Unidirectional(QuicDataReceiveStream),
}

/// A client-side QUIC data connection.
pub struct QuicDataClientConnection {
    endpoint: quinn::Endpoint,
    connection: quinn::Connection,
    max_payload_bytes: usize,
    expected_public_key_sha256: [u8; 32],
}

impl QuicDataClientConnection {
    #[must_use]
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Open a bounded bidirectional stream for a payload or relay operation.
    pub async fn open_stream(&self) -> Result<QuicDataStream, QuicDataError> {
        let (send, receive) = self
            .connection
            .open_bi()
            .await
            .map_err(|error| QuicDataError::Connection(error.to_string()))?;
        Ok(QuicDataStream {
            send,
            receive,
            max_payload_bytes: self.max_payload_bytes,
        })
    }

    pub fn close(&self) {
        self.connection.close(0_u32.into(), b"data client closed");
    }

    fn expected_public_key_sha256(&self) -> [u8; 32] {
        self.expected_public_key_sha256
    }

    fn into_parts(self) -> (quinn::Endpoint, quinn::Connection) {
        (self.endpoint, self.connection)
    }
}

/// A bounded QUIC data-plane client with frozen-style endpoint connection reuse.
///
/// The frozen client keeps one connection per endpoint and serializes creation
/// for each endpoint. Rust keeps the same observable reuse behavior while
/// bounding retained connections so a peer or changing endpoint list cannot
/// grow process memory without limit.
#[derive(Clone)]
pub struct QuicDataClient {
    state: Arc<Mutex<QuicDataClientState>>,
    max_payload_bytes: usize,
    max_cached_connections: usize,
}

struct QuicDataClientState {
    connections: HashMap<SocketAddr, Arc<QuicDataClientConnection>>,
    order: VecDeque<SocketAddr>,
    connecting: HashMap<SocketAddr, Arc<Mutex<()>>>,
}

impl QuicDataClientState {
    fn touch(&mut self, endpoint: SocketAddr) {
        self.order.retain(|candidate| *candidate != endpoint);
        self.order.push_back(endpoint);
    }

    fn remove_connection(&mut self, endpoint: SocketAddr) -> Option<Arc<QuicDataClientConnection>> {
        self.order.retain(|candidate| *candidate != endpoint);
        self.connections.remove(&endpoint)
    }
}

impl QuicDataClient {
    /// Create a client using the repository's bounded connection-cache limit.
    pub fn new(max_payload_bytes: usize) -> Self {
        Self::with_connection_limit(max_payload_bytes, DEFAULT_MAX_CACHED_CONNECTIONS)
    }

    /// Create a client with an explicit bounded endpoint-cache limit.
    pub fn with_connection_limit(max_payload_bytes: usize, max_cached_connections: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(QuicDataClientState {
                connections: HashMap::new(),
                order: VecDeque::new(),
                connecting: HashMap::new(),
            })),
            max_payload_bytes: max_payload_bytes.max(1),
            max_cached_connections: max_cached_connections.max(1),
        }
    }

    /// Send one bounded payload, reusing the pinned endpoint connection.
    pub async fn send(
        &self,
        endpoint: SocketAddr,
        expected_public_key_sha256: [u8; 32],
        payload: &[u8],
    ) -> Result<usize, QuicDataError> {
        if payload.len() > self.max_payload_bytes {
            return Err(QuicDataError::OversizedPayload {
                actual: payload.len(),
                max: self.max_payload_bytes,
            });
        }

        let connection = self
            .get_or_create(endpoint, expected_public_key_sha256)
            .await?;
        let result = async {
            let mut stream = connection.open_stream().await?;
            stream.write_payload(payload).await?;
            Ok(payload.len())
        }
        .await;
        if result.is_err() {
            self.remove_if_current(endpoint, &connection).await;
        }
        result
    }

    /// Open a bounded bidirectional stream over a reused endpoint connection.
    pub async fn open_bidirectional_stream(
        &self,
        endpoint: SocketAddr,
        expected_public_key_sha256: [u8; 32],
    ) -> Result<QuicDataStream, QuicDataError> {
        let connection = self
            .get_or_create(endpoint, expected_public_key_sha256)
            .await?;
        match connection.open_stream().await {
            Ok(stream) => Ok(stream),
            Err(error) => {
                self.remove_if_current(endpoint, &connection).await;
                Err(error)
            }
        }
    }

    /// Return the number of retained endpoint connections.
    pub async fn cached_connection_count(&self) -> usize {
        self.state.lock().await.connections.len()
    }

    /// Close and remove every retained endpoint connection.
    pub async fn close_all(&self) {
        let connections = {
            let mut state = self.state.lock().await;
            state.order.clear();
            state.connecting.clear();
            state
                .connections
                .drain()
                .map(|(_, connection)| connection)
                .collect::<Vec<_>>()
        };
        for connection in connections {
            connection.close();
        }
    }

    async fn get_or_create(
        &self,
        endpoint: SocketAddr,
        expected_public_key_sha256: [u8; 32],
    ) -> Result<Arc<QuicDataClientConnection>, QuicDataError> {
        let gate = {
            let mut state = self.state.lock().await;
            if let Some(connection) = state.connections.get(&endpoint).cloned() {
                if connection.expected_public_key_sha256() == expected_public_key_sha256 {
                    state.touch(endpoint);
                    return Ok(connection);
                }
                if let Some(stale) = state.remove_connection(endpoint) {
                    stale.close();
                }
            }

            state
                .connecting
                .retain(|_, gate| Arc::strong_count(gate) > 1);
            if !state.connecting.contains_key(&endpoint)
                && state.connecting.len() >= self.max_cached_connections
            {
                return Err(QuicDataError::CacheCapacity {
                    max: self.max_cached_connections,
                });
            }
            Arc::clone(
                state
                    .connecting
                    .entry(endpoint)
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };

        let guard = gate.lock().await;
        let result = self
            .connect_under_gate(endpoint, expected_public_key_sha256)
            .await;
        drop(guard);

        let mut state = self.state.lock().await;
        if state
            .connecting
            .get(&endpoint)
            .is_some_and(|current| Arc::ptr_eq(current, &gate) && Arc::strong_count(current) == 2)
        {
            state.connecting.remove(&endpoint);
        }
        result
    }

    async fn connect_under_gate(
        &self,
        endpoint: SocketAddr,
        expected_public_key_sha256: [u8; 32],
    ) -> Result<Arc<QuicDataClientConnection>, QuicDataError> {
        {
            let mut state = self.state.lock().await;
            if let Some(connection) = state.connections.get(&endpoint).cloned() {
                if connection.expected_public_key_sha256() == expected_public_key_sha256 {
                    state.touch(endpoint);
                    return Ok(connection);
                }
                if let Some(stale) = state.remove_connection(endpoint) {
                    stale.close();
                }
            }
        }

        let created = Arc::new(
            connect_quic_data(endpoint, expected_public_key_sha256, self.max_payload_bytes).await?,
        );
        let mut selected = Arc::clone(&created);
        let mut discarded = None;
        let mut evicted = Vec::new();
        {
            let mut state = self.state.lock().await;
            if let Some(existing) = state.connections.get(&endpoint).cloned() {
                if existing.expected_public_key_sha256() == expected_public_key_sha256 {
                    selected = existing;
                    discarded = Some(Arc::clone(&created));
                } else if let Some(stale) = state.remove_connection(endpoint) {
                    evicted.push(stale);
                }
            }
            if discarded.is_none() {
                state.connections.insert(endpoint, Arc::clone(&created));
                state.touch(endpoint);
                while state.connections.len() > self.max_cached_connections {
                    let Some(oldest) = state.order.pop_front() else {
                        break;
                    };
                    if oldest == endpoint {
                        state.order.push_back(oldest);
                        break;
                    }
                    if let Some(connection) = state.connections.remove(&oldest) {
                        evicted.push(connection);
                    }
                }
            }
        }
        if let Some(connection) = discarded {
            connection.close();
        }
        for connection in evicted {
            connection.close();
        }
        Ok(selected)
    }

    async fn remove_if_current(
        &self,
        endpoint: SocketAddr,
        expected: &Arc<QuicDataClientConnection>,
    ) {
        let removed = {
            let mut state = self.state.lock().await;
            let is_current = state
                .connections
                .get(&endpoint)
                .is_some_and(|current| Arc::ptr_eq(current, expected));
            is_current
                .then(|| state.remove_connection(endpoint))
                .flatten()
        };
        if let Some(connection) = removed {
            connection.close();
        }
    }
}

/// One bounded bidirectional data stream.
pub struct QuicDataStream {
    send: quinn::SendStream,
    receive: quinn::RecvStream,
    max_payload_bytes: usize,
}

impl QuicDataStream {
    /// Split the bidirectional stream for concurrent read/write handling.
    pub fn split(self) -> (QuicDataSendStream, QuicDataReceiveStream) {
        (
            QuicDataSendStream {
                send: self.send,
                max_payload_bytes: self.max_payload_bytes,
            },
            QuicDataReceiveStream {
                receive: self.receive,
                max_payload_bytes: self.max_payload_bytes,
            },
        )
    }

    /// Read the stream through the configured payload cap.
    pub async fn read_payload(self) -> Result<Vec<u8>, QuicDataError> {
        let (_, mut receive) = self.split();
        receive.read_to_end().await
    }

    /// Write a bounded response payload and finish the stream.
    pub async fn write_payload(&mut self, payload: &[u8]) -> Result<(), QuicDataError> {
        if payload.len() > self.max_payload_bytes {
            return Err(QuicDataError::OversizedPayload {
                actual: payload.len(),
                max: self.max_payload_bytes,
            });
        }
        self.send
            .write_all(payload)
            .await
            .map_err(|error| QuicDataError::Stream(error.to_string()))?;
        self.send
            .finish()
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }
}

/// The send half of a bounded QUIC data stream.
pub struct QuicDataSendStream {
    send: quinn::SendStream,
    max_payload_bytes: usize,
}

impl QuicDataSendStream {
    pub async fn write_all(&mut self, payload: &[u8]) -> Result<(), QuicDataError> {
        if payload.len() > self.max_payload_bytes {
            return Err(QuicDataError::OversizedPayload {
                actual: payload.len(),
                max: self.max_payload_bytes,
            });
        }
        self.send
            .write_all(payload)
            .await
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }

    pub async fn write_payload(&mut self, payload: &[u8]) -> Result<(), QuicDataError> {
        self.write_all(payload).await?;
        self.finish()
    }

    pub fn finish(&mut self) -> Result<(), QuicDataError> {
        self.send
            .finish()
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }
}

/// The receive half of a bounded QUIC data stream.
pub struct QuicDataReceiveStream {
    receive: quinn::RecvStream,
    max_payload_bytes: usize,
}

impl QuicDataReceiveStream {
    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
        self.receive
            .read(buffer)
            .await
            .map(|read| read.unwrap_or(0))
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }

    pub async fn read_to_end(&mut self) -> Result<Vec<u8>, QuicDataError> {
        self.receive
            .read_to_end(self.max_payload_bytes)
            .await
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }

    pub async fn read_to_end_after(
        &mut self,
        already_read: usize,
    ) -> Result<Vec<u8>, QuicDataError> {
        self.receive
            .read_to_end(self.max_payload_bytes.saturating_sub(already_read))
            .await
            .map_err(|error| QuicDataError::Stream(error.to_string()))
    }
}

/// Send one bounded raw payload through the frozen slskdN QUIC data protocol.
pub async fn send_quic_data(
    endpoint: SocketAddr,
    payload: &[u8],
    expected_public_key_sha256: [u8; 32],
) -> Result<usize, QuicDataError> {
    send_quic_data_with_limit(
        endpoint,
        payload,
        expected_public_key_sha256,
        DEFAULT_MAX_PAYLOAD_BYTES,
    )
    .await
}

/// Send one raw payload with an explicit peer-specific bound.
pub async fn send_quic_data_with_limit(
    endpoint: SocketAddr,
    payload: &[u8],
    expected_public_key_sha256: [u8; 32],
    max_payload_bytes: usize,
) -> Result<usize, QuicDataError> {
    let max_payload_bytes = max_payload_bytes.max(1);
    if payload.len() > max_payload_bytes {
        return Err(QuicDataError::OversizedPayload {
            actual: payload.len(),
            max: max_payload_bytes,
        });
    }

    let client = connect_quic_data(endpoint, expected_public_key_sha256, max_payload_bytes).await?;
    let mut stream = client.open_stream().await?;
    stream.write_payload(payload).await?;

    let length = payload.len();
    tokio::spawn(async move {
        tokio::time::sleep(QUIC_SEND_GRACE).await;
        let (endpoint_client, connection) = client.into_parts();
        connection.close(0_u32.into(), b"data sent");
        endpoint_client.wait_idle().await;
    });
    Ok(length)
}

/// Connect to a pinned QUIC data endpoint and retain it for multiple streams.
pub async fn connect_quic_data(
    endpoint: SocketAddr,
    expected_public_key_sha256: [u8; 32],
    max_payload_bytes: usize,
) -> Result<QuicDataClientConnection, QuicDataError> {
    let provider = rustls::crypto::ring::default_provider();
    let verifier = PublicKeyPinVerifier {
        signature_algorithms: provider.signature_verification_algorithms,
        expected_public_key_sha256,
    };
    let mut client_crypto = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| QuicDataError::Transport(error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![QUIC_DATA_ALPN.to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(client_crypto)
            .map_err(|error| QuicDataError::Transport(error.to_string()))?,
    ));
    let mut endpoint_client = quinn::Endpoint::client("[::]:0".parse().map_err(|error| {
        QuicDataError::Transport(format!("QUIC data client bind address failed: {error}"))
    })?)
    .map_err(|error| QuicDataError::Transport(error.to_string()))?;
    endpoint_client.set_default_client_config(client_config);

    let connecting = endpoint_client
        .connect(endpoint, "slskdn-overlay-data")
        .map_err(|error| QuicDataError::Transport(error.to_string()))?;
    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| QuicDataError::Timeout("QUIC data connect"))?
        .map_err(|error| QuicDataError::Transport(error.to_string()))?;
    Ok(QuicDataClientConnection {
        endpoint: endpoint_client,
        connection,
        max_payload_bytes: max_payload_bytes.max(1),
        expected_public_key_sha256,
    })
}

#[derive(Debug)]
struct PublicKeyPinVerifier {
    signature_algorithms: WebPkiSupportedAlgorithms,
    expected_public_key_sha256: [u8; 32],
}

impl ServerCertVerifier for PublicKeyPinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual = certificate_public_key_pin(end_entity.as_ref()).map_err(|error| {
            rustls::Error::General(format!("QUIC data certificate pin parse failed: {error}"))
        })?;
        if actual != self.expected_public_key_sha256 {
            return Err(rustls::Error::General(
                "QUIC data server public-key pin mismatch".to_owned(),
            ));
        }
        let mut roots = RootCertStore::empty();
        roots.add(end_entity.clone())?;
        let parsed = ParsedCertificate::try_from(end_entity)?;
        rustls::client::verify_server_cert_signed_by_trust_anchor(
            &parsed,
            &roots,
            intermediates,
            now,
            self.signature_algorithms.all,
        )?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.signature_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.signature_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.signature_algorithms.supported_schemes()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QuicDataError {
    #[error("QUIC data connection failed: {0}")]
    Connection(String),
    #[error("QUIC data stream failed: {0}")]
    Stream(String),
    #[error("QUIC data transport failed: {0}")]
    Transport(String),
    #[error("QUIC data {0} timed out")]
    Timeout(&'static str),
    #[error("QUIC data payload is too large: {actual} > {max} bytes")]
    OversizedPayload { actual: usize, max: usize },
    #[error("QUIC data connection cache capacity reached: {max} endpoints")]
    CacheCapacity { max: usize },
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use rcgen::generate_simple_self_signed;
    use tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer;

    use super::{connect_quic_data, send_quic_data, QuicDataClient, QuicDataServer};

    #[tokio::test]
    async fn data_sender_round_trips_exact_payload_under_the_frozen_alpn() {
        let certificate = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate_der = certificate.cert.der().clone();
        let certificate_pin =
            crate::quic_control::certificate_public_key_pin(certificate_der.as_ref()).unwrap();
        let private_key = PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
        let server = QuicDataServer::bind(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            certificate_der,
            private_key,
            128,
        )
        .unwrap();
        let address = server.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("incoming QUIC data connection")
                .expect("QUIC data connection");
            let stream = connection.accept_stream().await.unwrap();
            stream.read_payload().await.unwrap()
        });

        let payload = b"bounded-quic-data";
        assert_eq!(
            send_quic_data(address, payload, certificate_pin)
                .await
                .unwrap(),
            payload.len()
        );
        assert_eq!(server.await.unwrap(), payload);
    }

    #[tokio::test]
    async fn data_sender_rejects_oversized_payload_before_connecting() {
        let error = send_quic_data(
            SocketAddr::from(([127, 0, 0, 1], 9)),
            &[7_u8; 513 * 1024],
            [0_u8; 32],
        )
        .await
        .expect_err("oversized data payload must fail before network I/O");
        assert!(error.to_string().contains("too large"));
    }

    #[tokio::test]
    async fn data_client_opens_multiple_bounded_bidirectional_streams() {
        let certificate = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate_der = certificate.cert.der().clone();
        let certificate_pin =
            crate::quic_control::certificate_public_key_pin(certificate_der.as_ref()).unwrap();
        let private_key = PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
        let server = QuicDataServer::bind(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            certificate_der,
            private_key,
            128,
        )
        .unwrap();
        let address = server.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("incoming QUIC data connection")
                .expect("QUIC data connection");
            let first = connection.accept_stream().await.unwrap();
            let first_payload = first.read_payload().await.unwrap();
            let second = connection.accept_stream().await.unwrap();
            let second_payload = second.read_payload().await.unwrap();
            (first_payload, second_payload)
        });

        let client = connect_quic_data(address, certificate_pin, 128)
            .await
            .unwrap();
        let mut first = client.open_stream().await.unwrap();
        first.write_payload(b"first").await.unwrap();
        let mut second = client.open_stream().await.unwrap();
        second.write_payload(b"second").await.unwrap();
        let received = server.await.unwrap();
        client.close();

        assert_eq!(received, (b"first".to_vec(), b"second".to_vec()));
    }

    #[tokio::test]
    async fn cached_data_client_reuses_one_connection_for_send_and_stream() {
        let certificate = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate_der = certificate.cert.der().clone();
        let certificate_pin =
            crate::quic_control::certificate_public_key_pin(certificate_der.as_ref()).unwrap();
        let private_key = PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
        let server = QuicDataServer::bind(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            certificate_der,
            private_key,
            128,
        )
        .unwrap();
        let address = server.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("incoming QUIC data connection")
                .expect("QUIC data connection");
            let first = connection.accept_stream().await.unwrap();
            let first_payload = first.read_payload().await.unwrap();
            let second = connection.accept_stream().await.unwrap();
            let second_payload = second.read_payload().await.unwrap();
            (first_payload, second_payload)
        });

        let client = QuicDataClient::with_connection_limit(128, 1);
        assert_eq!(
            client
                .send(address, certificate_pin, b"cached")
                .await
                .unwrap(),
            6
        );
        let mut stream = client
            .open_bidirectional_stream(address, certificate_pin)
            .await
            .unwrap();
        stream.write_payload(b"stream").await.unwrap();

        assert_eq!(client.cached_connection_count().await, 1);
        assert_eq!(
            server.await.unwrap(),
            (b"cached".to_vec(), b"stream".to_vec())
        );
        client.close_all().await;
        assert_eq!(client.cached_connection_count().await, 0);
    }
}
