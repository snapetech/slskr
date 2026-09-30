//! One QUIC endpoint for both overlay ALPNs on a shared public socket.
//!
//! Quinn owns connection IDs and handshake reassembly. Dispatch happens after
//! TLS authenticates the negotiated ALPN, not by decrypting a first datagram.

use crate::{
    quic_control::{QuicControlConnection, QuicControlError, QUIC_CONTROL_ALPN},
    quic_data::{QuicDataConnection, MAX_PAYLOAD_BYTES, QUIC_DATA_ALPN},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_rustls::rustls::{
    self,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};

pub struct SharedQuicServer {
    endpoint: quinn::Endpoint,
    max_payload_bytes: usize,
}

pub enum SharedQuicConnection {
    Control(QuicControlConnection),
    Data(QuicDataConnection),
}

impl SharedQuicConnection {
    pub fn remote_address(&self) -> SocketAddr {
        match self {
            Self::Control(connection) => connection.remote_address(),
            Self::Data(connection) => connection.remote_address(),
        }
    }
}

impl SharedQuicServer {
    pub fn with_socket(
        socket: Arc<dyn quinn::AsyncUdpSocket>,
        certificate: CertificateDer<'static>,
        private_key: PrivatePkcs8KeyDer<'static>,
        control_enabled: bool,
        data_enabled: bool,
        max_payload_bytes: usize,
        max_concurrent_streams: u32,
    ) -> Result<Self, QuicControlError> {
        if !control_enabled && !data_enabled {
            return Err(QuicControlError::Transport(
                "shared QUIC requires an enabled ALPN".into(),
            ));
        }
        let mut crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate],
                rustls::pki_types::PrivateKeyDer::Pkcs8(private_key),
            )
            .map_err(|error| QuicControlError::Transport(error.to_string()))?;
        if control_enabled {
            crypto.alpn_protocols.push(QUIC_CONTROL_ALPN.to_vec());
        }
        if data_enabled {
            crypto.alpn_protocols.push(QUIC_DATA_ALPN.to_vec());
        }
        let mut config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(crypto)
                .map_err(|error| QuicControlError::Transport(error.to_string()))?,
        ));
        let streams = quinn::VarInt::from_u32(max_concurrent_streams.clamp(1, 1024));
        let mut transport = quinn::TransportConfig::default();
        transport
            .max_concurrent_bidi_streams(streams)
            .max_concurrent_uni_streams(streams);
        config.transport_config(Arc::new(transport));
        config
            .max_incoming(128)
            .incoming_buffer_size(64 * 1024)
            .incoming_buffer_size_total(8 * 1024 * 1024);
        let endpoint = quinn::Endpoint::new_with_abstract_socket(
            quinn::EndpointConfig::default(),
            Some(config),
            socket,
            Arc::new(quinn::TokioRuntime),
        )
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
        Ok(Self {
            endpoint,
            max_payload_bytes: max_payload_bytes.clamp(1, MAX_PAYLOAD_BYTES),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, QuicControlError> {
        self.endpoint
            .local_addr()
            .map_err(|error| QuicControlError::Transport(error.to_string()))
    }

    pub async fn accept(&self) -> Option<Result<SharedQuicConnection, QuicControlError>> {
        let incoming = self.endpoint.accept().await?;
        Some(self.accept_connection(incoming).await)
    }

    async fn accept_connection(
        &self,
        incoming: quinn::Incoming,
    ) -> Result<SharedQuicConnection, QuicControlError> {
        let connection = tokio::time::timeout(Duration::from_secs(10), incoming)
            .await
            .map_err(|_| QuicControlError::Timeout("shared QUIC handshake"))?
            .map_err(|error| QuicControlError::Connection(error.to_string()))?;
        let protocol = connection
            .handshake_data()
            .and_then(|data| data.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
            .and_then(|data| data.protocol);
        match protocol.as_deref() {
            Some(QUIC_CONTROL_ALPN) => Ok(SharedQuicConnection::Control(QuicControlConnection {
                connection,
            })),
            Some(QUIC_DATA_ALPN) => Ok(SharedQuicConnection::Data(QuicDataConnection {
                connection,
                max_payload_bytes: self.max_payload_bytes,
            })),
            _ => {
                connection.close(0_u32.into(), b"unsupported overlay ALPN");
                Err(QuicControlError::Transport(
                    "unsupported shared QUIC ALPN".into(),
                ))
            }
        }
    }

    pub fn close(&self) {
        self.endpoint.close(0_u32.into(), b"shared listener closed");
    }
}

#[cfg(test)]
#[path = "shared_quic_server_tests.rs"]
mod tests;
