use std::{
    collections::BTreeMap,
    fmt, fs,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::Duration,
};

use rcgen::generate_simple_self_signed;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshHelloAck, MeshServiceCall,
    MeshServiceReply, OpenTunnelRequest, OpenTunnelResponse, OverlayFramer, Ping, Pong,
    TunnelDataRequest, TunnelDataResponse, FEATURE_MESH_SERVICE, MAX_OVERLAY_MESSAGE_BYTES,
    OVERLAY_MAGIC, OVERLAY_VERSION,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::{mpsc, Mutex, RwLock, Semaphore},
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
const REQUEST_FRESHNESS_SECONDS: u64 = 300;
const DESTINATION_RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const DESTINATION_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DESTINATION_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const INBOUND_BUFFER_CHUNKS: usize = 64;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;

pub struct Gateway {
    bind: SocketAddr,
    acceptor: TlsAcceptor,
    certificate_sha256: [u8; 32],
    listener: Mutex<Option<TcpListener>>,
    connections: Arc<Semaphore>,
    tunnels: RwLock<BTreeMap<String, Arc<Tunnel>>>,
    replay_nonces: Mutex<BTreeMap<(String, String), u64>>,
}

impl fmt::Debug for Gateway {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Gateway")
            .field("bind", &self.bind)
            .field("certificate_sha256", &hex::encode(self.certificate_sha256))
            .finish_non_exhaustive()
    }
}

impl Gateway {
    pub async fn load_or_create(bind: SocketAddr, state_dir: &Path) -> Result<Self, String> {
        let (certificate, private_key) = load_or_create_certificate(state_dir)?;
        let certificate_sha256 = Sha256::digest(certificate.as_ref()).into();
        let config =
            ServerConfig::builder_with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key.into())
                .map_err(|error| format!("overlay TLS configuration failed: {error}"))?;
        let listener = TcpListener::bind(bind)
            .await
            .map_err(|error| format!("overlay listener bind failed: {error}"))?;
        let bind = listener
            .local_addr()
            .map_err(|error| format!("overlay listener address failed: {error}"))?;
        Ok(Self {
            bind,
            acceptor: TlsAcceptor::from(Arc::new(config)),
            certificate_sha256,
            listener: Mutex::new(Some(listener)),
            connections: Arc::new(Semaphore::new(MAX_GATEWAY_CONNECTIONS)),
            tunnels: RwLock::new(BTreeMap::new()),
            replay_nonces: Mutex::new(BTreeMap::new()),
        })
    }

    #[must_use]
    pub const fn certificate_sha256(&self) -> [u8; 32] {
        self.certificate_sha256
    }

    #[must_use]
    pub const fn bind(&self) -> SocketAddr {
        self.bind
    }

    pub async fn run(self: Arc<Self>, state: Arc<super::AppState>) -> Result<(), String> {
        let listener = self
            .listener
            .lock()
            .await
            .take()
            .ok_or_else(|| "overlay listener is already running".to_owned())?;
        loop {
            let (tcp, _) = listener
                .accept()
                .await
                .map_err(|error| format!("overlay listener accept failed: {error}"))?;
            let Ok(permit) = Arc::clone(&self.connections).try_acquire_owned() else {
                continue;
            };
            let gateway = Arc::clone(&self);
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(error) = gateway.handle_connection(tcp, &state).await {
                    tracing::debug!(%error, "overlay gateway connection closed");
                }
            });
        }
    }

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
        let mut framer = OverlayFramer::new(tls);
        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
            .await
            .map_err(|_| "overlay hello timed out".to_owned())?
            .map_err(|error| format!("overlay hello failed: {error}"))?;
        hello
            .validate()
            .map_err(|error| format!("overlay hello rejected: {error}"))?;
        if !hello
            .features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SERVICE))
        {
            return Err("overlay peer does not advertise mesh_service".to_owned());
        }
        authenticate_overlay_peer(state, &hello, remote_address.ip(), &self.certificate_sha256)
            .await?;
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        let local_username = super::pod_request_peer_id(state)
            .await
            .ok_or_else(|| "local gateway identity is unavailable".to_owned())?;
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: local_username,
                features: vec![FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: Some(self.bind.port()),
                nonce_echo: hello.nonce,
            })
            .await
            .map_err(|error| format!("overlay acknowledgement failed: {error}"))?;

        let result = async {
            loop {
                let raw = timeout(OVERLAY_IDLE_TIMEOUT, framer.read_raw())
                    .await
                    .map_err(|_| "overlay connection was idle too long".to_owned())?
                    .map_err(|error| format!("overlay read failed: {error}"))?;
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
                    "ping" => {
                        let ping: Ping = serde_json::from_slice(&raw)
                            .map_err(|error| format!("overlay ping is invalid: {error}"))?;
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
                    "disconnect" => return Ok(()),
                    _ => return Err("unsupported overlay message type".to_owned()),
                }
            }
        }
        .await;
        self.remove_connection_tunnels(&connection_id).await;
        result
    }

    async fn handle_call(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &super::AppState,
    ) -> MeshServiceReply {
        let result = if call.magic != OVERLAY_MAGIC
            || call.message_type != "mesh_service_call"
            || call.version != OVERLAY_VERSION
            || call.service_name != "private-gateway"
            || call.correlation_id.trim().is_empty()
            || call.payload.len() > MAX_OVERLAY_MESSAGE_BYTES
        {
            Err((4, "Invalid service call".to_owned()))
        } else {
            match call.method.as_str() {
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
            }
        };
        match result {
            Ok(payload) => service_reply(call.correlation_id, 0, payload, None),
            Err((status, error)) => {
                service_reply(call.correlation_id, status, Vec::new(), Some(error))
            }
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
        tokio::spawn(async move {
            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
            while let Ok(read) = reader.read(&mut buffer).await {
                if read == 0 || incoming_tx.send(buffer[..read].to_vec()).await.is_err() {
                    break;
                }
            }
        });
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

async fn authenticate_overlay_peer(
    state: &super::AppState,
    hello: &MeshHello,
    remote_ip: IpAddr,
    gateway_certificate_sha256: &[u8; 32],
) -> Result<(), String> {
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
    if hello.auth_public_key.is_some() || hello.auth_signature.is_some() {
        hello
            .verify_authentication(&public_key, gateway_certificate_sha256)
            .map_err(|_| "overlay peer failed capability-key authentication".to_owned())?;
    }
    let expected = super::request_peer_endpoint(state, &hello.username)
        .await
        .map_err(|_| "overlay peer Soulseek endpoint is unavailable".to_owned())?;
    if remote_ip != IpAddr::V4(expected.ip) {
        return Err("overlay peer IP does not match its Soulseek endpoint".to_owned());
    }
    Ok(())
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

fn load_or_create_certificate(
    state_dir: &Path,
) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), String> {
    let certificate_path = state_dir.join("overlay-certificate.der");
    let private_key_path = state_dir.join("overlay-private-key.der");
    let certificate = read_identity_file(&certificate_path, "certificate")?;
    let private_key = read_identity_file(&private_key_path, "private key")?;
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
    write_secret(&certificate_path, &certificate)?;
    write_secret(&private_key_path, &private_key)?;
    Ok((
        CertificateDer::from(certificate),
        PrivatePkcs8KeyDer::from(private_key),
    ))
}

fn read_identity_file(path: &Path, label: &str) -> Result<Option<Vec<u8>>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("overlay {label} metadata failed: {error}")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    let bytes = fs::read(path).map_err(|error| format!("overlay {label} read failed: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("overlay {label} is empty"));
    }
    Ok(Some(bytes))
}

fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
    fs::write(&temporary, bytes)
        .map_err(|error| format!("overlay identity write failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("overlay identity permissions failed: {error}"))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("overlay identity publish failed: {error}"))
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

    #[tokio::test]
    async fn gateway_certificate_identity_is_durable() {
        let root = temporary_directory("gateway-identity");
        let first = Gateway::load_or_create("127.0.0.1:0".parse().unwrap(), &root)
            .await
            .unwrap();
        let second = Gateway::load_or_create("127.0.0.1:0".parse().unwrap(), &root)
            .await
            .unwrap();
        assert_eq!(first.certificate_sha256(), second.certificate_sha256());
        drop((first, second));
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
}
