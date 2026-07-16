use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;
use tokio_rustls::{
    client::TlsStream,
    rustls::{
        self,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::WebPkiSupportedAlgorithms,
        pki_types::{CertificateDer, ServerName, UnixTime},
        server::ParsedCertificate,
        ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    },
    TlsConnector,
};

pub const OVERLAY_MAGIC: &str = "SLSKDNM1";
pub const OVERLAY_VERSION: i32 = 1;
pub const MAX_OVERLAY_MESSAGE_BYTES: usize = 64 * 1024;
pub const FEATURE_MESH_SERVICE: &str = "mesh_service";
const MAX_HANDSHAKE_FEATURES: usize = 20;
const MAX_FEATURE_BYTES: usize = 32;
const MAX_USERNAME_BYTES: usize = 64;
const MAX_NONCE_BYTES: usize = 64;
const MAX_SERVICE_FIELD_BYTES: usize = 128;
const MAX_UNMATCHED_SERVICE_FRAMES: usize = 32;
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const PROTOCOL_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const SERVICE_CALL_TIMEOUT: Duration = Duration::from_secs(30);

pub type TlsOverlayClient = OverlayClient<TlsStream<TcpStream>>;

pub async fn connect_tls_overlay(
    endpoint: impl ToSocketAddrs,
    expected_certificate_sha256: Option<[u8; 32]>,
    hello: MeshHello,
) -> Result<TlsOverlayClient, OverlayError> {
    let tcp = timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(endpoint))
        .await
        .map_err(|_| OverlayError::Timeout("TCP connect"))??;
    let provider = rustls::crypto::ring::default_provider();
    let verifier = SelfSignedOverlayVerifier {
        signature_algorithms: provider.signature_verification_algorithms,
        expected_certificate_sha256,
    };
    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| OverlayError::Tls(error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from("slskdn-overlay")
        .map_err(|error| OverlayError::Tls(error.to_string()))?;
    let tls = timeout(TLS_HANDSHAKE_TIMEOUT, connector.connect(server_name, tcp))
        .await
        .map_err(|_| OverlayError::Timeout("TLS handshake"))?
        .map_err(|error| OverlayError::Tls(error.to_string()))?;
    let certificate_sha256 = tls
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|certificates| certificates.first())
        .map(|certificate| Sha256::digest(certificate.as_ref()).into())
        .ok_or_else(|| OverlayError::Tls("overlay server certificate is missing".to_owned()))?;
    let mut client = timeout(
        PROTOCOL_HANDSHAKE_TIMEOUT,
        OverlayClient::handshake(tls, hello),
    )
    .await
    .map_err(|_| OverlayError::Timeout("overlay protocol handshake"))??;
    client.remote_certificate_sha256 = Some(certificate_sha256);
    Ok(client)
}

#[derive(Debug)]
struct SelfSignedOverlayVerifier {
    signature_algorithms: WebPkiSupportedAlgorithms,
    expected_certificate_sha256: Option<[u8; 32]>,
}

impl ServerCertVerifier for SelfSignedOverlayVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual_certificate_sha256: [u8; 32] = Sha256::digest(end_entity.as_ref()).into();
        if self
            .expected_certificate_sha256
            .is_some_and(|expected| actual_certificate_sha256 != expected)
        {
            return Err(rustls::Error::General(
                "overlay server certificate fingerprint mismatch".to_owned(),
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
pub struct OverlayFramer<S> {
    stream: S,
}

impl<S> OverlayFramer<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    #[must_use]
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> OverlayFramer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn write<T: Serialize>(&mut self, message: &T) -> Result<(), OverlayError> {
        let payload = serde_json::to_vec(message)?;
        if payload.len() > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(payload.len()));
        }
        if payload.len() < 2 {
            return Err(OverlayError::FrameTooSmall(payload.len()));
        }
        let length =
            u32::try_from(payload.len()).map_err(|_| OverlayError::FrameTooLarge(payload.len()))?;
        self.stream.write_all(&length.to_be_bytes()).await?;
        self.stream.write_all(&payload).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn read_raw(&mut self) -> Result<Vec<u8>, OverlayError> {
        let mut header = [0_u8; 4];
        self.stream.read_exact(&mut header).await?;
        if header[0] == b'{' {
            return self.read_legacy_unframed(header).await;
        }
        let length = u32::from_be_bytes(header) as usize;
        if length < 2 {
            return Err(OverlayError::FrameTooSmall(length));
        }
        if length > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(length));
        }
        let mut payload = vec![0_u8; length];
        self.stream.read_exact(&mut payload).await?;
        Ok(payload)
    }

    pub async fn read<T: DeserializeOwned>(&mut self) -> Result<T, OverlayError> {
        Ok(serde_json::from_slice(&self.read_raw().await?)?)
    }

    async fn read_legacy_unframed(&mut self, header: [u8; 4]) -> Result<Vec<u8>, OverlayError> {
        let mut payload = header.to_vec();
        loop {
            match serde_json::from_slice::<Value>(&payload) {
                Ok(Value::Object(_)) => return Ok(payload),
                Ok(_) => return Err(OverlayError::InvalidJsonObject),
                Err(error) if error.is_eof() => {}
                Err(error) => return Err(OverlayError::Json(error)),
            }
            if payload.len() >= MAX_OVERLAY_MESSAGE_BYTES {
                return Err(OverlayError::FrameTooLarge(payload.len()));
            }
            payload.push(self.stream.read_u8().await?);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulseekPorts {
    pub peer: u16,
    pub file: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHello {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub username: String,
    pub features: Vec<String>,
    pub soulseek_ports: Option<SoulseekPorts>,
    pub overlay_port: Option<u16>,
    pub nonce: Option<String>,
}

impl MeshHello {
    pub fn new(
        username: impl Into<String>,
        features: Vec<String>,
        soulseek_ports: Option<SoulseekPorts>,
        overlay_port: Option<u16>,
        nonce: impl Into<String>,
    ) -> Result<Self, OverlayError> {
        let message = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_hello".to_owned(),
            version: OVERLAY_VERSION,
            username: username.into(),
            features,
            soulseek_ports,
            overlay_port,
            nonce: Some(nonce.into()),
        };
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_handshake(
            &self.magic,
            &self.message_type,
            "mesh_hello",
            self.version,
            &self.username,
            &self.features,
            self.nonce.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHelloAck {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub username: String,
    pub features: Vec<String>,
    pub soulseek_ports: Option<SoulseekPorts>,
    pub overlay_port: Option<u16>,
    pub nonce_echo: Option<String>,
}

impl MeshHelloAck {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_handshake(
            &self.magic,
            &self.message_type,
            "mesh_hello_ack",
            self.version,
            &self.username,
            &self.features,
            self.nonce_echo.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshServiceCall {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub correlation_id: String,
    pub service_name: String,
    pub method: String,
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
}

impl MeshServiceCall {
    pub fn new(
        correlation_id: impl Into<String>,
        service_name: impl Into<String>,
        method: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<Self, OverlayError> {
        let call = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_service_call".to_owned(),
            version: OVERLAY_VERSION,
            correlation_id: correlation_id.into(),
            service_name: service_name.into(),
            method: method.into(),
            payload,
        };
        validate_service_field("correlation_id", &call.correlation_id)?;
        validate_service_field("service_name", &call.service_name)?;
        validate_service_field("method", &call.method)?;
        Ok(call)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshServiceReply {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub correlation_id: String,
    pub status_code: i32,
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub struct OverlayClient<S> {
    framer: OverlayFramer<S>,
    pub remote_username: String,
    pub remote_features: Vec<String>,
    pub remote_overlay_port: Option<u16>,
    pub remote_certificate_sha256: Option<[u8; 32]>,
}

impl<S> OverlayClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn handshake(stream: S, hello: MeshHello) -> Result<Self, OverlayError> {
        hello.validate()?;
        let expected_nonce = hello.nonce.clone();
        let mut framer = OverlayFramer::new(stream);
        framer.write(&hello).await?;
        let acknowledgement: MeshHelloAck = framer.read().await?;
        acknowledgement.validate()?;
        if acknowledgement.nonce_echo != expected_nonce {
            return Err(OverlayError::NonceMismatch);
        }
        Ok(Self {
            framer,
            remote_username: acknowledgement.username,
            remote_features: acknowledgement.features,
            remote_overlay_port: acknowledgement.overlay_port,
            remote_certificate_sha256: None,
        })
    }

    pub async fn call(&mut self, call: &MeshServiceCall) -> Result<MeshServiceReply, OverlayError> {
        self.call_with_timeout(call, SERVICE_CALL_TIMEOUT).await
    }

    async fn call_with_timeout(
        &mut self,
        call: &MeshServiceCall,
        deadline: Duration,
    ) -> Result<MeshServiceReply, OverlayError> {
        timeout(deadline, self.call_inner(call))
            .await
            .map_err(|_| OverlayError::Timeout("overlay service call"))?
    }

    async fn call_inner(
        &mut self,
        call: &MeshServiceCall,
    ) -> Result<MeshServiceReply, OverlayError> {
        if !self
            .remote_features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SERVICE))
        {
            return Err(OverlayError::MeshServiceUnsupported);
        }
        self.framer.write(call).await?;
        for _ in 0..MAX_UNMATCHED_SERVICE_FRAMES {
            let payload = self.framer.read_raw().await?;
            let message_type = message_type(&payload)?;
            match message_type.as_str() {
                "mesh_service_reply" => {
                    let reply: MeshServiceReply = serde_json::from_slice(&payload)?;
                    validate_overlay_base(&reply.magic, &reply.message_type, reply.version)?;
                    if reply.correlation_id == call.correlation_id {
                        return Ok(reply);
                    }
                }
                "ping" => {
                    let ping: Ping = serde_json::from_slice(&payload)?;
                    validate_overlay_base(&ping.magic, &ping.message_type, ping.version)?;
                    self.framer
                        .write(&Pong {
                            magic: OVERLAY_MAGIC.to_owned(),
                            message_type: "pong".to_owned(),
                            version: OVERLAY_VERSION,
                            timestamp: ping.timestamp,
                        })
                        .await?;
                }
                "disconnect" => return Err(OverlayError::Disconnected),
                _ => {}
            }
        }
        Err(OverlayError::ReplyNotFound)
    }

    #[must_use]
    pub fn into_inner(self) -> S {
        self.framer.into_inner()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OpenTunnelRequest {
    pub pod_id: String,
    pub destination_host: String,
    pub destination_port: u16,
    pub service_name: Option<String>,
    pub request_nonce: String,
    pub request_timestamp: i64,
}

impl OpenTunnelRequest {
    pub fn new(
        pod_id: impl Into<String>,
        destination_host: impl Into<String>,
        destination_port: u16,
        service_name: Option<String>,
        request_nonce: impl Into<String>,
    ) -> Result<Self, OverlayError> {
        let request = Self {
            pod_id: pod_id.into(),
            destination_host: destination_host.into(),
            destination_port,
            service_name,
            request_nonce: request_nonce.into(),
            request_timestamp: unix_seconds()?,
        };
        if request.pod_id.trim().is_empty()
            || request.destination_host.trim().is_empty()
            || request.destination_port == 0
            || request.request_nonce.trim().is_empty()
        {
            return Err(OverlayError::InvalidPrivateGatewayRequest);
        }
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OpenTunnelResponse {
    pub tunnel_id: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TunnelDataRequest {
    pub tunnel_id: String,
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetTunnelDataRequest {
    pub tunnel_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TunnelDataResponse {
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
    pub bytes_received: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CloseTunnelRequest {
    pub tunnel_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OverlayError {
    #[error("overlay I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("overlay JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("overlay frame is too small: {0}")]
    FrameTooSmall(usize),
    #[error("overlay frame is too large: {0}")]
    FrameTooLarge(usize),
    #[error("overlay message must be a JSON object")]
    InvalidJsonObject,
    #[error("overlay protocol magic is invalid")]
    InvalidMagic,
    #[error("overlay protocol version {0} is invalid")]
    InvalidVersion(i32),
    #[error("overlay message type is invalid")]
    InvalidMessageType,
    #[error("overlay username is invalid")]
    InvalidUsername,
    #[error("overlay feature list is invalid")]
    InvalidFeatures,
    #[error("overlay nonce is invalid")]
    InvalidNonce,
    #[error("overlay handshake nonce does not match")]
    NonceMismatch,
    #[error("overlay service field {0} is invalid")]
    InvalidServiceField(&'static str),
    #[error("remote overlay does not advertise mesh_service")]
    MeshServiceUnsupported,
    #[error("overlay peer disconnected")]
    Disconnected,
    #[error("matching overlay service reply was not received")]
    ReplyNotFound,
    #[error("private-gateway request is invalid")]
    InvalidPrivateGatewayRequest,
    #[error("system clock is before the Unix epoch")]
    InvalidTime,
    #[error("overlay base64 payload is invalid")]
    InvalidBase64,
    #[error("overlay {0} timed out")]
    Timeout(&'static str),
    #[error("overlay TLS failed: {0}")]
    Tls(String),
}

fn message_type(payload: &[u8]) -> Result<String, OverlayError> {
    serde_json::from_slice::<Value>(payload)?
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(OverlayError::InvalidMessageType)
}

fn validate_handshake(
    magic: &str,
    message_type: &str,
    expected_type: &str,
    version: i32,
    username: &str,
    features: &[String],
    nonce: Option<&str>,
) -> Result<(), OverlayError> {
    validate_overlay_base(magic, message_type, version)?;
    if message_type != expected_type {
        return Err(OverlayError::InvalidMessageType);
    }
    if username.is_empty()
        || username.len() > MAX_USERNAME_BYTES
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(OverlayError::InvalidUsername);
    }
    if features.len() > MAX_HANDSHAKE_FEATURES
        || features.iter().any(|feature| {
            feature.is_empty()
                || feature.len() > MAX_FEATURE_BYTES
                || !feature
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(OverlayError::InvalidFeatures);
    }
    if nonce.is_some_and(|nonce| {
        nonce.is_empty()
            || nonce.len() > MAX_NONCE_BYTES
            || !nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    }) {
        return Err(OverlayError::InvalidNonce);
    }
    Ok(())
}

fn validate_overlay_base(
    magic: &str,
    message_type: &str,
    version: i32,
) -> Result<(), OverlayError> {
    if magic.as_bytes() != OVERLAY_MAGIC.as_bytes() {
        return Err(OverlayError::InvalidMagic);
    }
    if !(1..=100).contains(&version) {
        return Err(OverlayError::InvalidVersion(version));
    }
    if message_type.trim().is_empty() {
        return Err(OverlayError::InvalidMessageType);
    }
    Ok(())
}

fn validate_service_field(field: &'static str, value: &str) -> Result<(), OverlayError> {
    if value.trim().is_empty() || value.len() > MAX_SERVICE_FIELD_BYTES {
        Err(OverlayError::InvalidServiceField(field))
    } else {
        Ok(())
    }
}

fn unix_seconds() -> Result<i64, OverlayError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .ok_or(OverlayError::InvalidTime)
}

mod base64_bytes {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::generate_simple_self_signed;
    use tokio::io::duplex;
    use tokio::net::TcpListener;
    use tokio_rustls::{
        rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig},
        TlsAcceptor,
    };

    #[tokio::test]
    async fn framer_uses_big_endian_length_and_round_trips_json() {
        let (client, mut wire) = duplex(4096);
        let task = tokio::spawn(async move {
            OverlayFramer::new(client)
                .write(&serde_json::json!({"type":"ping"}))
                .await
                .unwrap();
        });
        let mut header = [0_u8; 4];
        wire.read_exact(&mut header).await.unwrap();
        assert_eq!(u32::from_be_bytes(header), 15);
        let mut payload = vec![0; 15];
        wire.read_exact(&mut payload).await.unwrap();
        assert_eq!(payload, br#"{"type":"ping"}"#);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn client_handshake_and_service_call_match_overlay_contract() {
        let (client, server) = duplex(16 * 1024);
        let server = tokio::spawn(async move {
            let mut framer = OverlayFramer::new(server);
            let hello: MeshHello = framer.read().await.unwrap();
            assert_eq!(hello.magic, OVERLAY_MAGIC);
            assert_eq!(hello.message_type, "mesh_hello");
            assert_eq!(hello.nonce.as_deref(), Some("nonce_1"));
            framer
                .write(&MeshHelloAck {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_hello_ack".to_owned(),
                    version: OVERLAY_VERSION,
                    username: "gateway".to_owned(),
                    features: vec![FEATURE_MESH_SERVICE.to_owned()],
                    soulseek_ports: None,
                    overlay_port: Some(50_305),
                    nonce_echo: hello.nonce,
                })
                .await
                .unwrap();
            let call: MeshServiceCall = framer.read().await.unwrap();
            assert_eq!(call.service_name, "private-gateway");
            assert_eq!(call.method, "TunnelData");
            let nested: TunnelDataRequest = serde_json::from_slice(&call.payload).unwrap();
            assert_eq!(nested.data, vec![0, 1, 2, 255]);
            framer
                .write(&MeshServiceReply {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_service_reply".to_owned(),
                    version: OVERLAY_VERSION,
                    correlation_id: call.correlation_id,
                    status_code: 0,
                    payload: br#"{"Sent":4}"#.to_vec(),
                    error_message: None,
                })
                .await
                .unwrap();
        });

        let hello = MeshHello::new(
            "local",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            "nonce_1",
        )
        .unwrap();
        let mut client = OverlayClient::handshake(client, hello).await.unwrap();
        assert_eq!(client.remote_username, "gateway");
        let nested = serde_json::to_vec(&TunnelDataRequest {
            tunnel_id: "tunnel".to_owned(),
            data: vec![0, 1, 2, 255],
        })
        .unwrap();
        let call =
            MeshServiceCall::new("correlation", "private-gateway", "TunnelData", nested).unwrap();
        let reply = client.call(&call).await.unwrap();
        assert_eq!(reply.status_code, 0);
        assert_eq!(reply.payload, br#"{"Sent":4}"#);
        server.await.unwrap();
    }

    #[test]
    fn byte_arrays_use_system_text_json_base64_shape() {
        let call =
            MeshServiceCall::new("c", "private-gateway", "TunnelData", vec![0, 1, 2, 255]).unwrap();
        let json = serde_json::to_value(call).unwrap();
        assert_eq!(json["payload"], "AAEC/w==");

        let nested = serde_json::to_value(TunnelDataRequest {
            tunnel_id: "t".to_owned(),
            data: vec![0, 1, 2, 255],
        })
        .unwrap();
        assert_eq!(nested["TunnelId"], "t");
        assert_eq!(nested["Data"], "AAEC/w==");
    }

    #[tokio::test]
    async fn framer_rejects_oversized_declared_length_before_allocation() {
        let (mut sender, receiver) = duplex(16);
        sender
            .write_all(&((MAX_OVERLAY_MESSAGE_BYTES as u32) + 1).to_be_bytes())
            .await
            .unwrap();
        let error = OverlayFramer::new(receiver).read_raw().await.unwrap_err();
        assert!(matches!(error, OverlayError::FrameTooLarge(_)));
    }

    #[tokio::test]
    async fn tls_overlay_accepts_frozen_runtime_style_self_signed_certificate() {
        let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let certificate_sha256 = Sha256::digest(certificate.as_ref()).into();
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key.into())
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let tls = acceptor.accept(tcp).await.unwrap();
            let mut framer = OverlayFramer::new(tls);
            let hello: MeshHello = framer.read().await.unwrap();
            framer
                .write(&MeshHelloAck {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_hello_ack".to_owned(),
                    version: OVERLAY_VERSION,
                    username: "gateway".to_owned(),
                    features: vec![FEATURE_MESH_SERVICE.to_owned()],
                    soulseek_ports: None,
                    overlay_port: Some(address.port()),
                    nonce_echo: hello.nonce,
                })
                .await
                .unwrap();
        });
        let hello = MeshHello::new(
            "local",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            "tls_nonce",
        )
        .unwrap();
        let client = connect_tls_overlay(address, certificate_sha256, hello)
            .await
            .unwrap();
        assert_eq!(client.remote_username, "gateway");
        assert_eq!(client.remote_overlay_port, Some(address.port()));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn tls_overlay_rejects_an_unpinned_self_signed_certificate() {
        let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key.into())
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            assert!(acceptor.accept(tcp).await.is_err());
        });
        let hello = MeshHello::new("local", Vec::new(), None, None, "tls_nonce").unwrap();
        let error = connect_tls_overlay(address, [0; 32], hello)
            .await
            .unwrap_err();
        assert!(matches!(error, OverlayError::Tls(_)));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn service_call_deadline_covers_a_silent_peer() {
        let (client, server) = duplex(16 * 1024);
        let server = tokio::spawn(async move {
            let mut framer = OverlayFramer::new(server);
            let hello: MeshHello = framer.read().await.unwrap();
            framer
                .write(&MeshHelloAck {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_hello_ack".to_owned(),
                    version: OVERLAY_VERSION,
                    username: "gateway".to_owned(),
                    features: vec![FEATURE_MESH_SERVICE.to_owned()],
                    soulseek_ports: None,
                    overlay_port: None,
                    nonce_echo: hello.nonce,
                })
                .await
                .unwrap();
            let _: MeshServiceCall = framer.read().await.unwrap();
            std::future::pending::<()>().await;
        });
        let hello = MeshHello::new(
            "local",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            "nonce",
        )
        .unwrap();
        let mut client = OverlayClient::handshake(client, hello).await.unwrap();
        let call = MeshServiceCall::new("c", "private-gateway", "OpenTunnel", Vec::new()).unwrap();
        let error = client
            .call_with_timeout(&call, Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            OverlayError::Timeout("overlay service call")
        ));
        server.abort();
    }
}
