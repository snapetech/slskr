use super::frame::OverlayFramer;
use super::messages::{message_type, validate_mesh_search_response_limit};
use super::*;

pub type TlsOverlayClient = OverlayClient<TlsStream<TcpStream>>;

pub async fn connect_tls_overlay(
    endpoint: impl ToSocketAddrs,
    expected_certificate_sha256: [u8; 32],
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
    expected_certificate_sha256: [u8; 32],
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
        if actual_certificate_sha256 != self.expected_certificate_sha256 {
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
pub struct OverlayClient<S> {
    framer: OverlayFramer<S>,
    failed: bool,
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
            failed: false,
            remote_username: acknowledgement.username,
            remote_features: acknowledgement.features,
            remote_overlay_port: acknowledgement.overlay_port,
            remote_certificate_sha256: None,
        })
    }

    pub async fn call(&mut self, call: &MeshServiceCall) -> Result<MeshServiceReply, OverlayError> {
        self.call_with_timeout(call, SERVICE_CALL_TIMEOUT).await
    }

    pub async fn search(
        &mut self,
        request: &MeshSearchRequestMessage,
    ) -> Result<MeshSearchResponseMessage, OverlayError> {
        self.search_with_timeout(request, SERVICE_CALL_TIMEOUT)
            .await
    }

    pub async fn call_with_timeout(
        &mut self,
        call: &MeshServiceCall,
        deadline: Duration,
    ) -> Result<MeshServiceReply, OverlayError> {
        if self.failed {
            return Err(OverlayError::Disconnected);
        }
        call.validate()?;
        if !self
            .remote_features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SERVICE))
        {
            return Err(OverlayError::MeshServiceUnsupported);
        }
        match timeout(deadline, self.call_inner(call)).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(error)) => {
                self.failed = true;
                Err(error)
            }
            Err(_) => {
                self.failed = true;
                Err(OverlayError::Timeout("overlay service call"))
            }
        }
    }

    async fn call_inner(
        &mut self,
        call: &MeshServiceCall,
    ) -> Result<MeshServiceReply, OverlayError> {
        self.framer.write(call).await?;
        let mut unmatched_frames = 0;
        let mut control_frames = 0;
        while unmatched_frames < MAX_UNMATCHED_SERVICE_FRAMES
            && control_frames < MAX_SERVICE_CONTROL_FRAMES
        {
            let payload = self.framer.read_raw().await?;
            let message_type = message_type(&payload)?;
            match message_type.as_str() {
                "mesh_service_reply" => {
                    let reply: MeshServiceReply = serde_json::from_slice(&payload)?;
                    reply.validate()?;
                    if reply.correlation_id == call.correlation_id {
                        return Ok(reply);
                    }
                    unmatched_frames += 1;
                }
                "ping" => {
                    let ping: Ping = serde_json::from_slice(&payload)?;
                    ping.validate()?;
                    control_frames += 1;
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
                _ => unmatched_frames += 1,
            }
        }
        Err(OverlayError::ReplyNotFound)
    }

    pub async fn search_with_timeout(
        &mut self,
        request: &MeshSearchRequestMessage,
        deadline: Duration,
    ) -> Result<MeshSearchResponseMessage, OverlayError> {
        if self.failed {
            return Err(OverlayError::Disconnected);
        }
        request.validate()?;
        if !self
            .remote_features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SEARCH))
        {
            return Err(OverlayError::MeshSearchUnsupported);
        }
        match timeout(deadline, self.search_inner(request)).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(error)) => {
                self.failed = true;
                Err(error)
            }
            Err(_) => {
                self.failed = true;
                Err(OverlayError::Timeout("overlay mesh search"))
            }
        }
    }

    async fn search_inner(
        &mut self,
        request: &MeshSearchRequestMessage,
    ) -> Result<MeshSearchResponseMessage, OverlayError> {
        self.framer.write(request).await?;
        let mut unmatched_frames = 0;
        let mut control_frames = 0;
        while unmatched_frames < MAX_UNMATCHED_SERVICE_FRAMES
            && control_frames < MAX_SERVICE_CONTROL_FRAMES
        {
            let payload = self.framer.read_raw().await?;
            let message_type = message_type(&payload)?;
            match message_type.as_str() {
                "mesh_search_resp" => {
                    let response: MeshSearchResponseMessage = serde_json::from_slice(&payload)?;
                    response.validate()?;
                    validate_mesh_search_response_limit(&response, request.max_results)?;
                    if response.request_id == request.request_id {
                        return Ok(response);
                    }
                    unmatched_frames += 1;
                }
                "ping" => {
                    let ping: Ping = serde_json::from_slice(&payload)?;
                    ping.validate()?;
                    control_frames += 1;
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
                _ => unmatched_frames += 1,
            }
        }
        Err(OverlayError::ReplyNotFound)
    }

    #[must_use]
    pub fn into_inner(self) -> S {
        self.framer.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::super::messages::validate_overlay_base;
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
    async fn legacy_framer_scans_large_json_once() {
        let payload = serde_json::to_vec(&serde_json::json!({
            "payload": format!("{}{{escaped}}", "x".repeat(60_000)),
        }))
        .unwrap();
        assert!(payload.len() < MAX_OVERLAY_MESSAGE_BYTES);
        let (mut wire, client) = duplex(4_096);
        let expected = payload.clone();
        let writer = tokio::spawn(async move {
            wire.write_all(&payload).await.unwrap();
        });

        let decoded = timeout(
            Duration::from_secs(2),
            OverlayFramer::new(client).read_raw(),
        )
        .await
        .expect("legacy frame scan must remain linear")
        .unwrap();

        assert_eq!(decoded, expected);
        writer.await.unwrap();
    }

    #[test]
    fn ping_and_pong_validation_bounds_frozen_control_timestamps() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let ping = Ping {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "ping".to_owned(),
            version: OVERLAY_VERSION,
            timestamp: now,
        };
        assert!(ping.validate().is_ok());
        assert!(Pong {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "pong".to_owned(),
            version: OVERLAY_VERSION,
            timestamp: now,
        }
        .validate()
        .is_ok());

        let mut invalid = ping.clone();
        invalid.timestamp -= i64::try_from(CONTROL_TIMESTAMP_SKEW_MILLIS).unwrap() + 1;
        assert!(matches!(
            invalid.validate(),
            Err(OverlayError::InvalidControlTimestamp)
        ));
        invalid = ping;
        invalid.message_type = "pong".to_owned();
        assert!(matches!(
            invalid.validate(),
            Err(OverlayError::InvalidMessageType)
        ));
    }

    #[test]
    fn overlay_messages_reject_non_contract_protocol_versions() {
        for version in [OVERLAY_VERSION - 1, OVERLAY_VERSION + 1, 100] {
            assert!(matches!(
                validate_overlay_base(OVERLAY_MAGIC, "mesh_service_reply", version),
                Err(OverlayError::InvalidVersion(rejected)) if rejected == version
            ));
            assert!(matches!(
                MeshHelloAck {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_hello_ack".to_owned(),
                    version,
                    username: "peer".to_owned(),
                    features: vec![FEATURE_MESH_SERVICE.to_owned()],
                    soulseek_ports: None,
                    overlay_port: None,
                    nonce_echo: Some("nonce".to_owned()),
                }
                .validate(),
                Err(OverlayError::InvalidVersion(rejected)) if rejected == version
            ));
        }
    }

    #[test]
    fn overlay_handshakes_reject_zero_advertised_ports() {
        for (soulseek_ports, overlay_port, field) in [
            (
                Some(SoulseekPorts {
                    peer: 0,
                    file: 22_35,
                }),
                None,
                "Soulseek peer",
            ),
            (
                Some(SoulseekPorts {
                    peer: 22_34,
                    file: 0,
                }),
                None,
                "Soulseek file",
            ),
            (None, Some(0), "overlay"),
        ] {
            let hello = MeshHello::new(
                "peer",
                vec![FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports.clone(),
                overlay_port,
                "nonce",
            );
            assert!(matches!(
                hello,
                Err(OverlayError::InvalidAdvertisedPort(rejected)) if rejected == field
            ));
            let acknowledgement = MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "peer".to_owned(),
                features: vec![FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports,
                overlay_port,
                nonce_echo: Some("nonce".to_owned()),
            };
            assert!(matches!(
                acknowledgement.validate(),
                Err(OverlayError::InvalidAdvertisedPort(rejected)) if rejected == field
            ));
        }
    }

    #[test]
    fn mesh_hello_authentication_binds_identity_capabilities_endpoints_and_key() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let gateway_certificate_sha256 = [3; 32];
        let mut hello = MeshHello::new(
            "member",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            "unique_nonce",
        )
        .unwrap();
        assert!(matches!(
            hello.verify_authentication(
                &signing_key.verifying_key().to_bytes(),
                &gateway_certificate_sha256,
            ),
            Err(OverlayError::InvalidPeerAuthentication)
        ));
        hello
            .authenticate(&signing_key, &gateway_certificate_sha256)
            .unwrap();
        hello
            .verify_authentication(
                &signing_key.verifying_key().to_bytes(),
                &gateway_certificate_sha256,
            )
            .unwrap();

        let encoded = serde_json::to_vec(&hello).unwrap();
        let mut decoded: MeshHello = serde_json::from_slice(&encoded).unwrap();
        decoded
            .verify_authentication(
                &signing_key.verifying_key().to_bytes(),
                &gateway_certificate_sha256,
            )
            .unwrap();
        decoded.username = "impostor".to_owned();
        assert!(matches!(
            decoded.verify_authentication(
                &signing_key.verifying_key().to_bytes(),
                &gateway_certificate_sha256,
            ),
            Err(OverlayError::InvalidPeerAuthentication)
        ));

        for tampered in [
            {
                let mut tampered = hello.clone();
                tampered.features.push("other".to_owned());
                tampered
            },
            {
                let mut tampered = hello.clone();
                tampered.soulseek_ports = Some(SoulseekPorts {
                    peer: 2234,
                    file: 2235,
                });
                tampered
            },
            {
                let mut tampered = hello.clone();
                tampered.overlay_port = Some(8443);
                tampered
            },
        ] {
            assert!(matches!(
                tampered.verify_authentication(
                    &signing_key.verifying_key().to_bytes(),
                    &gateway_certificate_sha256,
                ),
                Err(OverlayError::InvalidPeerAuthentication)
            ));
        }
        assert!(matches!(
            hello.verify_authentication(
                &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes(),
                &gateway_certificate_sha256,
            ),
            Err(OverlayError::InvalidPeerAuthentication)
        ));
        assert!(matches!(
            hello.verify_authentication(&signing_key.verifying_key().to_bytes(), &[4; 32],),
            Err(OverlayError::InvalidPeerAuthentication)
        ));
    }

    #[test]
    fn mesh_hello_authentication_rejects_weak_public_key_forgery() {
        let mut public_key = [0_u8; 32];
        public_key[0] = 1;
        let mut signature = vec![0_u8; 64];
        signature[0] = 1;
        let gateway_certificate_sha256 = [3; 32];
        let mut hello = MeshHello::new(
            "peer",
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            Some(2234),
            "nonce",
        )
        .unwrap();
        hello.auth_public_key = Some(public_key);
        hello.auth_signature = Some(signature);

        assert!(matches!(
            hello.verify_authentication(&public_key, &gateway_certificate_sha256),
            Err(OverlayError::InvalidPeerAuthentication)
        ));
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

    #[tokio::test]
    async fn service_call_revalidates_mutated_public_fields_before_write() {
        let (client, mut wire) = duplex(1024);
        let mut overlay = OverlayClient {
            framer: OverlayFramer::new(client),
            failed: false,
            remote_username: "gateway".to_owned(),
            remote_features: vec![FEATURE_MESH_SERVICE.to_owned()],
            remote_overlay_port: None,
            remote_certificate_sha256: None,
        };
        let mut call =
            MeshServiceCall::new("correlation", "private-gateway", "OpenTunnel", Vec::new())
                .unwrap();
        call.correlation_id = "   ".to_owned();

        assert!(matches!(
            overlay.call(&call).await.unwrap_err(),
            OverlayError::InvalidServiceField("correlation_id")
        ));
        assert!(timeout(Duration::from_millis(10), wire.read_u8())
            .await
            .is_err());

        for (field, call) in [
            (
                "correlation_id",
                MeshServiceCall::new(
                    "correlation\nforged",
                    "private-gateway",
                    "OpenTunnel",
                    Vec::new(),
                ),
            ),
            (
                "service_name",
                MeshServiceCall::new(
                    "correlation",
                    "private-gateway\rforged",
                    "OpenTunnel",
                    Vec::new(),
                ),
            ),
            (
                "method",
                MeshServiceCall::new(
                    "correlation",
                    "private-gateway",
                    "OpenTunnel\0forged",
                    Vec::new(),
                ),
            ),
        ] {
            assert!(matches!(
                call,
                Err(OverlayError::InvalidServiceField(rejected)) if rejected == field
            ));
        }
    }

    #[test]
    fn service_replies_reject_malformed_remote_fields() {
        let valid = MeshServiceReply {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_service_reply".to_owned(),
            version: OVERLAY_VERSION,
            correlation_id: "correlation".to_owned(),
            status_code: 0,
            payload: Vec::new(),
            error_message: None,
        };
        valid.validate().expect("valid service reply");

        let mut invalid_type = valid.clone();
        invalid_type.message_type = "ping".to_owned();
        assert!(matches!(
            invalid_type.validate(),
            Err(OverlayError::InvalidMessageType)
        ));

        for correlation_id in [
            "   ".to_owned(),
            "forged\r\ncorrelation".to_owned(),
            "x".repeat(MAX_SERVICE_FIELD_BYTES + 1),
        ] {
            let mut invalid = valid.clone();
            invalid.correlation_id = correlation_id;
            assert!(matches!(
                invalid.validate(),
                Err(OverlayError::InvalidServiceField("correlation_id"))
            ));
        }

        for error_message in [
            "forged\r\nterminal entry".to_owned(),
            "x".repeat(MAX_SERVICE_ERROR_BYTES + 1),
        ] {
            let mut invalid = valid.clone();
            invalid.error_message = Some(error_message);
            assert!(matches!(
                invalid.validate(),
                Err(OverlayError::InvalidServiceField("error_message"))
            ));
        }
    }

    #[test]
    fn overlay_dtos_bound_payloads_and_reject_control_text() {
        assert!(matches!(
            MeshServiceCall::new(
                "correlation",
                "private-gateway",
                "OpenTunnel",
                vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
            ),
            Err(OverlayError::FrameTooLarge(length))
                if length == MAX_OVERLAY_MESSAGE_BYTES + 1
        ));

        let reply = MeshServiceReply {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_service_reply".to_owned(),
            version: OVERLAY_VERSION,
            correlation_id: "correlation".to_owned(),
            status_code: 0,
            payload: vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
            error_message: None,
        };
        assert!(matches!(
            reply.validate(),
            Err(OverlayError::FrameTooLarge(length))
                if length == MAX_OVERLAY_MESSAGE_BYTES + 1
        ));

        let mut request = MeshSearchRequestMessage::new(
            "00000000-0000-0000-0000-000000000001",
            "search",
            10,
            Some("scope".to_owned()),
        )
        .unwrap();
        request.search_text.push('\n');
        assert!(matches!(
            request.validate(),
            Err(OverlayError::InvalidMeshSearchRequest("search_text"))
        ));
        request.search_text = "search".to_owned();
        request.scope = Some("scope\rforged".to_owned());
        assert!(matches!(
            request.validate(),
            Err(OverlayError::InvalidMeshSearchRequest("scope"))
        ));

        let response = MeshSearchResponseMessage::new(
            "00000000-0000-0000-0000-000000000001",
            vec![MeshSearchFileDto {
                filename: "file\nforged".to_owned(),
                size: 1,
                extension: None,
                bitrate: None,
                duration: None,
                codec: None,
                media_kinds: None,
                content_id: None,
                hash: None,
            }],
            false,
            None,
        );
        assert!(matches!(
            response,
            Err(OverlayError::InvalidMeshSearchResponse("file"))
        ));

        let response = MeshSearchResponseMessage::new(
            "00000000-0000-0000-0000-000000000001",
            vec![
                MeshSearchFileDto {
                    filename: "first.flac".to_owned(),
                    size: 1,
                    extension: Some("flac".to_owned()),
                    bitrate: None,
                    duration: None,
                    codec: None,
                    media_kinds: None,
                    content_id: None,
                    hash: None,
                },
                MeshSearchFileDto {
                    filename: "second.flac".to_owned(),
                    size: 2,
                    extension: Some("flac".to_owned()),
                    bitrate: None,
                    duration: None,
                    codec: None,
                    media_kinds: None,
                    content_id: None,
                    hash: None,
                },
            ],
            false,
            None,
        )
        .unwrap();
        assert!(matches!(
            validate_mesh_search_response_limit(&response, 1),
            Err(OverlayError::InvalidMeshSearchResponse("files"))
        ));
        validate_mesh_search_response_limit(&response, 2).unwrap();

        let disconnect = Disconnect {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "disconnect".to_owned(),
            version: OVERLAY_VERSION,
            reason: Some("bye\rforged".to_owned()),
        };
        assert!(matches!(
            disconnect.validate(),
            Err(OverlayError::InvalidDisconnectReason)
        ));
    }

    #[tokio::test]
    async fn service_call_control_pings_do_not_exhaust_reply_budget() {
        let (client, server) = duplex(64 * 1024);
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
            let call: MeshServiceCall = framer.read().await.unwrap();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            for _ in 0..MAX_UNMATCHED_SERVICE_FRAMES {
                framer
                    .write(&Ping {
                        magic: OVERLAY_MAGIC.to_owned(),
                        message_type: "ping".to_owned(),
                        version: OVERLAY_VERSION,
                        timestamp,
                    })
                    .await
                    .unwrap();
                let pong: Pong = framer.read().await.unwrap();
                assert_eq!(pong.timestamp, timestamp);
            }
            framer
                .write(&MeshServiceReply {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_service_reply".to_owned(),
                    version: OVERLAY_VERSION,
                    correlation_id: call.correlation_id,
                    status_code: 0,
                    payload: Vec::new(),
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
            "nonce",
        )
        .unwrap();
        let mut client = OverlayClient::handshake(client, hello).await.unwrap();
        let call = MeshServiceCall::new("c", "private-gateway", "OpenTunnel", Vec::new()).unwrap();

        assert_eq!(client.call(&call).await.unwrap().status_code, 0);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn service_call_bounds_control_ping_floods() {
        let (client, server) = duplex(128 * 1024);
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
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
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            for _ in 0..MAX_SERVICE_CONTROL_FRAMES {
                framer
                    .write(&Ping {
                        magic: OVERLAY_MAGIC.to_owned(),
                        message_type: "ping".to_owned(),
                        version: OVERLAY_VERSION,
                        timestamp,
                    })
                    .await
                    .unwrap();
                let pong: Pong = framer.read().await.unwrap();
                assert_eq!(pong.timestamp, timestamp);
            }
            let _ = done_rx.await;
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

        let result = client.call(&call).await;
        let _ = done_tx.send(());
        assert!(matches!(result, Err(OverlayError::ReplyNotFound)));
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

    #[test]
    fn private_gateway_request_constructor_enforces_gateway_field_limits() {
        let valid = || {
            OpenTunnelRequest::new(
                "p".repeat(MAX_POD_ID_BYTES),
                "h".repeat(MAX_DESTINATION_HOST_BYTES),
                80,
                Some("s".repeat(MAX_SERVICE_FIELD_BYTES)),
                "n".repeat(MAX_NONCE_BYTES),
            )
        };
        assert!(valid().is_ok());

        let invalid = [
            OpenTunnelRequest::new("p".repeat(MAX_POD_ID_BYTES + 1), "host", 80, None, "nonce"),
            OpenTunnelRequest::new(
                "pod",
                "h".repeat(MAX_DESTINATION_HOST_BYTES + 1),
                80,
                None,
                "nonce",
            ),
            OpenTunnelRequest::new(
                "pod",
                "host",
                80,
                Some("s".repeat(MAX_SERVICE_FIELD_BYTES + 1)),
                "nonce",
            ),
            OpenTunnelRequest::new("pod", "host", 80, Some(" ".to_owned()), "nonce"),
            OpenTunnelRequest::new("pod", "host", 80, None, "n".repeat(MAX_NONCE_BYTES + 1)),
            OpenTunnelRequest::new("pod\nforged", "host", 80, None, "nonce"),
            OpenTunnelRequest::new("pod", "host\r\nforged", 80, None, "nonce"),
            OpenTunnelRequest::new(
                "pod",
                "host",
                80,
                Some("service\tforged".to_owned()),
                "nonce",
            ),
            OpenTunnelRequest::new("pod", "host", 80, None, "nonce\0forged"),
        ];
        assert!(invalid
            .into_iter()
            .all(|request| matches!(request, Err(OverlayError::InvalidPrivateGatewayRequest))));
    }

    #[test]
    fn private_gateway_request_deserialization_enforces_gateway_field_limits() {
        let request = serde_json::json!({
            "PodId": "pod",
            "DestinationHost": "host",
            "DestinationPort": 80,
            "ServiceName": "service",
            "RequestNonce": "nonce",
            "RequestTimestamp": 1_700_000_000_i64,
        });
        assert!(serde_json::from_value::<OpenTunnelRequest>(request.clone()).is_ok());

        for (field, value) in [
            ("PodId", "pod\nforged"),
            ("DestinationHost", "host\r\nforged"),
            ("ServiceName", "service\tforged"),
            ("RequestNonce", "nonce\0forged"),
        ] {
            let mut invalid = request.clone();
            invalid[field] = serde_json::Value::String(value.to_owned());
            assert!(serde_json::from_value::<OpenTunnelRequest>(invalid).is_err());
        }
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
        assert!(matches!(
            client
                .call_with_timeout(&call, Duration::from_secs(1))
                .await
                .unwrap_err(),
            OverlayError::Disconnected
        ));
        server.abort();
    }
}
