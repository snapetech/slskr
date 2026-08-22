//! QUIC control-plane transport for slskdN overlay envelopes.
//!
//! The frozen slskdN runtime uses one bidirectional QUIC stream per control
//! envelope.  The stream contains the same MessagePack `ControlEnvelope` as
//! the UDP path; QUIC adds reliable delivery and the `slskdn-overlay` ALPN.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use quinn::crypto::rustls::QuicClientConfig;
use sha2::Digest;
use tokio::time::timeout;
use tokio_rustls::rustls::{
    self,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::WebPkiSupportedAlgorithms,
    pki_types::{CertificateDer, ServerName, UnixTime},
    server::ParsedCertificate,
    DigitallySignedStruct, RootCertStore, SignatureScheme,
};

use crate::overlay_control::{ControlEnvelope, ControlEnvelopeError, CONTROL_MAX_DATAGRAM_BYTES};

pub const QUIC_CONTROL_ALPN: &[u8] = b"slskdn-overlay";
const QUIC_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUIC_SEND_GRACE: Duration = Duration::from_millis(100);

/// Compute the target's certificate pin: SHA-256 over the DER-encoded
/// SubjectPublicKeyInfo key-value (the `PublicKey.EncodedKeyValue.RawData`
/// value used by slskdN), not over the complete certificate.
pub fn certificate_public_key_pin(certificate_der: &[u8]) -> Result<[u8; 32], QuicControlError> {
    let (_, certificate) =
        x509_parser::parse_x509_certificate(certificate_der).map_err(|error| {
            QuicControlError::Transport(format!("certificate parse failed: {error}"))
        })?;
    let key = &certificate.tbs_certificate.subject_pki.subject_public_key;
    let key_value_len = key.data.len().saturating_add(1);
    let mut encoded = Vec::with_capacity(key_value_len + 5);
    encoded.push(0x03); // DER BIT STRING tag.
    write_der_length(&mut encoded, key_value_len)?;
    encoded.push(key.unused_bits);
    encoded.extend_from_slice(&key.data);
    Ok(sha2::Sha256::digest(encoded).into())
}

fn write_der_length(output: &mut Vec<u8>, length: usize) -> Result<(), QuicControlError> {
    if length > u32::MAX as usize {
        return Err(QuicControlError::Transport(
            "certificate public key is too large".to_owned(),
        ));
    }
    match length {
        0..=127 => output.push(length as u8),
        128..=255 => {
            output.push(0x81);
            output.push(length as u8);
        }
        256..=65_535 => {
            output.push(0x82);
            output.extend_from_slice(&(length as u16).to_be_bytes());
        }
        _ => {
            output.push(0x83);
            output.extend_from_slice(&(length as u32).to_be_bytes()[1..]);
        }
    }
    Ok(())
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
            rustls::Error::General(format!("QUIC certificate pin parse failed: {error}"))
        })?;
        if actual != self.expected_public_key_sha256 {
            return Err(rustls::Error::General(
                "QUIC server public-key pin mismatch".to_owned(),
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

#[derive(Debug)]
struct CertificatePinCaptureVerifier {
    signature_algorithms: WebPkiSupportedAlgorithms,
    captured_public_key_sha256: Arc<Mutex<Option<[u8; 32]>>>,
}

impl ServerCertVerifier for CertificatePinCaptureVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual = certificate_public_key_pin(end_entity.as_ref()).map_err(|error| {
            rustls::Error::General(format!("QUIC certificate pin parse failed: {error}"))
        })?;
        let mut captured = self.captured_public_key_sha256.lock().map_err(|_| {
            rustls::Error::General("QUIC certificate pin capture lock poisoned".to_owned())
        })?;
        *captured = Some(actual);
        drop(captured);

        // The frozen target creates a self-signed certificate for every
        // process. This bounded discovery handshake trusts only that one
        // certificate long enough to capture its exact SPKI pin, while still
        // requiring the certificate to be a valid self-signed certificate.
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

/// Discover the public-key pin of a bounded, self-signed slskdN QUIC
/// endpoint. This is intended for the interop probe against the frozen target,
/// whose QUIC servers generate a new certificate at process start and do not
/// publish that certificate through its HTTP API. Production peer connections
/// must continue to use a preconfigured pin with [`send_quic_control`].
pub async fn discover_certificate_public_key_pin(
    endpoint: SocketAddr,
    alpn: &[u8],
    server_name: &str,
) -> Result<[u8; 32], QuicControlError> {
    let captured_public_key_sha256 = Arc::new(Mutex::new(None));
    let provider = rustls::crypto::ring::default_provider();
    let verifier = CertificatePinCaptureVerifier {
        signature_algorithms: provider.signature_verification_algorithms,
        captured_public_key_sha256: Arc::clone(&captured_public_key_sha256),
    };
    let mut client_crypto = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| QuicControlError::Transport(error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![alpn.to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(client_crypto)
            .map_err(|error| QuicControlError::Transport(error.to_string()))?,
    ));
    let mut endpoint_client = quinn::Endpoint::client("[::]:0".parse().map_err(|error| {
        QuicControlError::Transport(format!("QUIC pin-discovery bind address failed: {error}"))
    })?)
    .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    endpoint_client.set_default_client_config(client_config);

    let connecting = endpoint_client
        .connect(endpoint, server_name)
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| QuicControlError::Timeout("QUIC certificate pin discovery"))?
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    connection.close(0_u32.into(), b"certificate pin discovered");
    endpoint_client.wait_idle().await;

    let captured = captured_public_key_sha256
        .lock()
        .map_err(|_| QuicControlError::Transport("QUIC pin capture lock poisoned".to_owned()))?
        .to_owned();
    captured.ok_or_else(|| {
        QuicControlError::Transport("QUIC handshake did not expose a server certificate".to_owned())
    })
}

/// A bounded QUIC control-plane listener using the frozen slskdN ALPN and
/// certificate shape.
pub struct QuicControlServer {
    endpoint: quinn::Endpoint,
}

impl QuicControlServer {
    /// Bind a QUIC control listener with the supplied self-signed identity.
    pub fn bind(
        bind: SocketAddr,
        certificate: rustls::pki_types::CertificateDer<'static>,
        private_key: rustls::pki_types::PrivatePkcs8KeyDer<'static>,
    ) -> Result<Self, QuicControlError> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate],
                rustls::pki_types::PrivateKeyDer::Pkcs8(private_key),
            )
            .map_err(|error| QuicControlError::Transport(error.to_string()))?;
        server_crypto.alpn_protocols = vec![QUIC_CONTROL_ALPN.to_vec()];
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .map_err(|error| QuicControlError::Transport(error.to_string()))?,
        ));
        let endpoint = quinn::Endpoint::server(server_config, bind)
            .map_err(|error| QuicControlError::Transport(error.to_string()))?;
        Ok(Self { endpoint })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, QuicControlError> {
        self.endpoint
            .local_addr()
            .map_err(|error| QuicControlError::Transport(error.to_string()))
    }

    /// Accept the next QUIC connection. `None` means the endpoint was closed.
    pub async fn accept(&self) -> Option<Result<QuicControlConnection, QuicControlError>> {
        let incoming = self.endpoint.accept().await?;
        Some(
            incoming
                .await
                .map(|connection| QuicControlConnection { connection })
                .map_err(|error| QuicControlError::Connection(error.to_string())),
        )
    }

    pub fn close(&self) {
        self.endpoint
            .close(0_u32.into(), b"control listener closed");
    }
}

/// One accepted QUIC control connection.
pub struct QuicControlConnection {
    connection: quinn::Connection,
}

impl QuicControlConnection {
    #[must_use]
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Read one bounded bidirectional stream as one ControlEnvelope.
    pub async fn accept_envelope(&self) -> Result<ControlEnvelope, QuicControlError> {
        let (_send, mut receive) = self
            .connection
            .accept_bi()
            .await
            .map_err(|error| QuicControlError::Connection(error.to_string()))?;
        let payload = receive
            .read_to_end(CONTROL_MAX_DATAGRAM_BYTES)
            .await
            .map_err(|error| QuicControlError::Stream(error.to_string()))?;
        ControlEnvelope::decode(&payload).map_err(QuicControlError::Envelope)
    }
}

/// Send one signed control envelope through the frozen slskdN QUIC control
/// protocol.
pub async fn send_quic_control(
    endpoint: SocketAddr,
    envelope: &ControlEnvelope,
    expected_public_key_sha256: [u8; 32],
) -> Result<(), QuicControlError> {
    let payload = envelope.encode()?;
    if payload.len() > CONTROL_MAX_DATAGRAM_BYTES {
        return Err(QuicControlError::OversizedEnvelope(payload.len()));
    }

    let provider = rustls::crypto::ring::default_provider();
    let verifier = PublicKeyPinVerifier {
        signature_algorithms: provider.signature_verification_algorithms,
        expected_public_key_sha256,
    };
    let mut client_crypto = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| QuicControlError::Transport(error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![QUIC_CONTROL_ALPN.to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(client_crypto)
            .map_err(|error| QuicControlError::Transport(error.to_string()))?,
    ));
    let mut endpoint_client = quinn::Endpoint::client("[::]:0".parse().map_err(|error| {
        QuicControlError::Transport(format!("QUIC client bind address failed: {error}"))
    })?)
    .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    endpoint_client.set_default_client_config(client_config);

    let connecting = endpoint_client
        .connect(endpoint, "slskdn-overlay")
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| QuicControlError::Timeout("QUIC connect"))?
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    let (mut send, _receive) = connection
        .open_bi()
        .await
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    send.write_all(&payload)
        .await
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    send.finish()
        .map_err(|error| QuicControlError::Transport(error.to_string()))?;
    // The frozen client keeps a connection cache alive.  This one-shot helper
    // cannot expose that cache, so retain the connection briefly in a cleanup
    // task; closing it immediately can race the peer's accept loop and drop a
    // perfectly-written stream before the envelope is observed.
    tokio::spawn(async move {
        tokio::time::sleep(QUIC_SEND_GRACE).await;
        connection.close(0_u32.into(), b"envelope sent");
        endpoint_client.wait_idle().await;
    });
    Ok(())
}

/// Errors from the QUIC control transport.
#[derive(Debug, thiserror::Error)]
pub enum QuicControlError {
    #[error(transparent)]
    Envelope(#[from] ControlEnvelopeError),
    #[error("QUIC control connection failed: {0}")]
    Connection(String),
    #[error("QUIC control stream failed: {0}")]
    Stream(String),
    #[error("QUIC control transport failed: {0}")]
    Transport(String),
    #[error("QUIC control {0} timed out")]
    Timeout(&'static str),
    #[error("QUIC control envelope is too large: {0} bytes")]
    OversizedEnvelope(usize),
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::Arc};

    use ed25519_dalek::SigningKey;
    use rcgen::generate_simple_self_signed;
    use tokio_rustls::rustls::{self, pki_types::PrivatePkcs8KeyDer};

    use super::{send_quic_control, QuicControlServer};
    use crate::overlay_control::ControlEnvelope;

    #[tokio::test]
    async fn quic_sender_emits_target_alpn_and_messagepack_stream() {
        let certificate = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate_der = certificate.cert.der().clone();
        let certificate_pin = super::certificate_public_key_pin(certificate_der.as_ref()).unwrap();
        let private_key = PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
        let server = QuicControlServer::bind(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            certificate_der,
            private_key,
        )
        .unwrap();
        let address = server.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("incoming QUIC connection")
                .expect("QUIC connection");
            connection.accept_envelope().await.unwrap()
        });

        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let envelope = ControlEnvelope::signed_at(
            "probe",
            b"probe-payload".to_vec(),
            "quic-message",
            2,
            &signing_key,
        )
        .unwrap();
        send_quic_control(address, &envelope, certificate_pin)
            .await
            .unwrap();
        let received = server.await.unwrap();
        assert_eq!(received, envelope);
        received.verify().unwrap();
    }

    #[tokio::test]
    async fn quic_sender_rejects_a_wrong_certificate_pin_before_writing() {
        let certificate = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate_der = certificate.cert.der().clone();
        let private_key = PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate_der],
                rustls::pki_types::PrivateKeyDer::Pkcs8(private_key),
            )
            .unwrap();
        server_crypto.alpn_protocols = vec![b"slskdn-overlay".to_vec()];
        let endpoint = quinn::Endpoint::server(
            quinn::ServerConfig::with_crypto(Arc::new(
                quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto).unwrap(),
            )),
            SocketAddr::from(([127, 0, 0, 1], 0)),
        )
        .unwrap();
        let address = endpoint.local_addr().unwrap();
        let server = tokio::spawn(async move {
            if let Some(incoming) = endpoint.accept().await {
                let _ = incoming.await;
            }
        });
        let signing_key = SigningKey::from_bytes(&[8_u8; 32]);
        let envelope =
            ControlEnvelope::signed_at("probe", Vec::new(), "wrong-pin", 2, &signing_key).unwrap();
        let error = send_quic_control(address, &envelope, [0_u8; 32])
            .await
            .expect_err("wrong certificate pin must fail closed");
        assert!(error.to_string().contains("certificate") || error.to_string().contains("QUIC"));
        server.abort();
    }
}
