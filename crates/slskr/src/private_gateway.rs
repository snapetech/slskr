use std::{
    collections::BTreeMap,
    fmt, fs,
    io::{Read as _, Seek as _, SeekFrom},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
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
const MAX_CERTIFICATE_BYTES: u64 = 64 * 1024;
const MAX_PRIVATE_KEY_BYTES: u64 = 16 * 1024;
const REQUEST_FRESHNESS_SECONDS: u64 = 300;
const DESTINATION_RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const DESTINATION_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DESTINATION_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(2 * 60);
const OVERLAY_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const INBOUND_BUFFER_CHUNKS: usize = 64;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;
const MAX_POD_MESSAGE_BODY_BYTES: usize = 4 * 1024;

struct OverlayLiveness {
    last_inbound: Instant,
    last_ping: Instant,
}

impl OverlayLiveness {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last_inbound: now,
            last_ping: now,
        }
    }

    fn record_inbound(&mut self) {
        self.last_inbound = Instant::now();
    }

    fn record_ping(&mut self) {
        self.last_ping = Instant::now();
    }

    fn is_idle(&self) -> bool {
        self.last_inbound.elapsed() >= OVERLAY_IDLE_TIMEOUT
    }

    fn read_wait(&self) -> Duration {
        OVERLAY_MESSAGE_READ_TIMEOUT.min(
            OVERLAY_KEEPALIVE_INTERVAL
                .checked_sub(self.last_ping.elapsed())
                .unwrap_or(Duration::ZERO),
        )
    }
}
const MAX_MESH_CONTENT_BYTES: usize = 32 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 512;
const MAX_SHADOW_MBID_BYTES: usize = 100;
const MAX_SHADOW_BATCH: usize = 20;

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
            let mut liveness = OverlayLiveness::new();
            loop {
                if liveness.is_idle() {
                    return Err("overlay connection was idle too long".to_owned());
                }
                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
                    Ok(result) => {
                        liveness.record_inbound();
                        result.map_err(|error| format!("overlay read failed: {error}"))?
                    }
                    Err(_) if liveness.last_ping.elapsed() >= OVERLAY_KEEPALIVE_INTERVAL => {
                        let timestamp = i64::try_from(super::unix_timestamp_millis())
                            .map_err(|_| "overlay clock is out of range".to_owned())?;
                        framer
                            .write(&Ping {
                                magic: OVERLAY_MAGIC.to_owned(),
                                message_type: "ping".to_owned(),
                                version: OVERLAY_VERSION,
                                timestamp,
                            })
                            .await
                            .map_err(|error| format!("overlay keepalive failed: {error}"))?;
                        liveness.record_ping();
                        continue;
                    }
                    Err(_) => continue,
                };
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
                        ping.validate()
                            .map_err(|_| "overlay ping is invalid".to_owned())?;
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
                    "pong" => {
                        let pong: Pong = serde_json::from_slice(&raw)
                            .map_err(|error| format!("overlay pong is invalid: {error}"))?;
                        pong.validate()
                            .map_err(|_| "overlay pong is invalid".to_owned())?;
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
            || call.correlation_id.trim().is_empty()
            || call.payload.len() > MAX_OVERLAY_MESSAGE_BYTES
        {
            Err((4, "Invalid service call".to_owned()))
        } else {
            match call.service_name.as_str() {
                "private-gateway" => match call.method.as_str() {
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
                },
                "pods" => {
                    self.handle_pods_call(&call.method, &call.payload, remote_username, state)
                        .await
                }
                "shadow-index" => {
                    self.handle_shadow_index_call(&call.method, &call.payload, state)
                        .await
                }
                "MeshContent" => {
                    self.handle_mesh_content_call(&call.method, &call.payload, state)
                        .await
                }
                _ => Err((2, "Unknown service".to_owned())),
            }
        };
        match result {
            Ok(payload) => service_reply(call.correlation_id, 0, payload, None),
            Err((status, error)) => {
                service_reply(call.correlation_id, status, Vec::new(), Some(error))
            }
        }
    }

    async fn handle_shadow_index_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        match method {
            "QueryByMbid" => {
                let request: ShadowQueryRequest = parse_payload(payload)?;
                let mbid = valid_shadow_mbid(&request.mbid)?;
                let result = shadow_index_result(state, mbid)
                    .await
                    .ok_or_else(|| (2, "No data found for MBID".to_owned()))?;
                serde_json::to_vec(&result)
                    .map_err(|_| (1, "Shadow-index response failed".to_owned()))
            }
            "QueryBatch" => {
                let request: ShadowBatchRequest = parse_payload(payload)?;
                if request.mbids.is_empty() || request.mbids.len() > MAX_SHADOW_BATCH {
                    return Err((
                        if request.mbids.len() > MAX_SHADOW_BATCH {
                            9
                        } else {
                            4
                        },
                        "MBIDs list is invalid".to_owned(),
                    ));
                }
                let mut results = serde_json::Map::new();
                let mut seen = std::collections::HashSet::new();
                for mbid in request.mbids {
                    let mbid = valid_shadow_mbid(&mbid)?;
                    if !seen.insert(mbid.to_owned()) {
                        continue;
                    }
                    if let Some(result) = shadow_index_result(state, mbid).await {
                        results.insert(mbid.to_owned(), result);
                    }
                }
                serde_json::to_vec(&results)
                    .map_err(|_| (1, "Shadow-index response failed".to_owned()))
            }
            _ => Err((3, "Unknown method".to_owned())),
        }
    }

    async fn handle_mesh_content_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        if method != "GetByContentId" {
            return Err((3, "Unknown method".to_owned()));
        }
        let request: MeshContentRequest = parse_payload(payload)?;
        let content_id = bounded_required(&request.content_id, MAX_CONTENT_ID_BYTES, "ContentId")?;
        let (local_path, indexed_size) = {
            let shares = state.shares.read().await;
            let entry = shares
                .entries
                .iter()
                .find(|entry| {
                    entry.filename == content_id
                        || super::stable_content_hash(&entry.filename, entry.size).to_string()
                            == content_id
                })
                .ok_or_else(|| (2, "Content not found or not advertisable".to_owned()))?;
            let local_path = shares
                .local_paths
                .get(&entry.filename)
                .cloned()
                .ok_or_else(|| (2, "Content not found or not advertisable".to_owned()))?;
            (local_path, entry.size)
        };
        let mut file = super::open_shared_local_file(state, &local_path)
            .map_err(|_| (2, "Content not found or not advertisable".to_owned()))?;
        let actual_size = file
            .metadata()
            .map_err(|_| (10, "Content metadata failed".to_owned()))?
            .len();
        if actual_size != indexed_size || actual_size == 0 {
            return Err((2, "Content not found or not advertisable".to_owned()));
        }
        let (offset, length) = mesh_content_range(request.range.as_ref(), actual_size)?;
        let bytes = tokio::task::spawn_blocking(move || {
            file.seek(SeekFrom::Start(offset))?;
            let mut bytes = vec![0_u8; length];
            file.read_exact(&mut bytes)?;
            Ok::<_, std::io::Error>(bytes)
        })
        .await
        .map_err(|_| (10, "Content read task failed".to_owned()))?
        .map_err(|_| (10, "Content read failed".to_owned()))?;
        Ok(bytes)
    }

    async fn handle_pods_call(
        &self,
        method: &str,
        payload: &[u8],
        remote_username: &str,
        state: &super::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        match method {
            "List" => serde_json::to_vec(&state.pods.read().await.list_visible(None))
                .map_err(|_| (1, "Pod response failed".to_owned())),
            "Get" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let pods = state.pods.read().await;
                let pod = pods
                    .get(pod_id)
                    .filter(|_| pods.is_public(pod_id) || pods.is_member(pod_id, remote_username))
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&pod).map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "Join" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let joined = state
                    .pods
                    .write()
                    .await
                    .join(pod_id, remote_username.to_owned())
                    .map_err(|error| (8, error))?
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&serde_json::json!({"Success": joined}))
                    .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "Leave" => {
                let request: PodIdRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let left = state
                    .pods
                    .write()
                    .await
                    .leave(pod_id, remote_username)
                    .map_err(|error| (8, error))?
                    .ok_or_else(|| (2, "Pod not found".to_owned()))?;
                serde_json::to_vec(&serde_json::json!({"Success": left}))
                    .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "PostMessage" => {
                let request: PodMessageRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let channel_id =
                    bounded_required(&request.channel_id, MAX_POD_ID_BYTES, "ChannelId")?;
                if request.body.trim().is_empty() || request.body.len() > MAX_POD_MESSAGE_BODY_BYTES
                {
                    return Err((9, "Message body is invalid".to_owned()));
                }
                let binding = {
                    let pods = state.pods.read().await;
                    if !pods.channel_exists(pod_id, channel_id) {
                        return Err((2, "Pod channel not found".to_owned()));
                    }
                    if !pods.is_member(pod_id, remote_username) {
                        return Err((8, "Pod membership is required".to_owned()));
                    }
                    pods.soulseek_binding(pod_id, channel_id)
                };
                let message = state
                    .pod_channels
                    .write()
                    .await
                    .append(
                        pod_id.to_owned(),
                        channel_id.to_owned(),
                        remote_username.to_owned(),
                        request.body,
                        request.signature.unwrap_or_default(),
                        super::unix_timestamp_millis(),
                    )
                    .map_err(|error| (1, error))?;
                if let Some(binding) =
                    binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
                {
                    let _ = super::try_send_session_command(
                        state,
                        super::SessionCommand::SayRoom {
                            room: binding.identifier,
                            body: format!("[Pod:{}] {}", message.sender_peer_id, message.body),
                        },
                    );
                }
                serde_json::to_vec(&serde_json::json!({
                    "Success": true,
                    "MessageId": message.message_id,
                }))
                .map_err(|_| (1, "Pod response failed".to_owned()))
            }
            "GetMessages" => {
                let request: PodMessagesRequest = parse_payload(payload)?;
                let pod_id = bounded_required(&request.pod_id, MAX_POD_ID_BYTES, "PodId")?;
                let channel_id =
                    bounded_required(&request.channel_id, MAX_POD_ID_BYTES, "ChannelId")?;
                let pods = state.pods.read().await;
                if !pods.channel_exists(pod_id, channel_id) {
                    return Err((2, "Pod channel not found".to_owned()));
                }
                if !pods.is_member(pod_id, remote_username) {
                    return Err((8, "Pod membership is required".to_owned()));
                }
                drop(pods);
                let since = match request.since_timestamp {
                    Some(value) => Some(
                        u64::try_from(value)
                            .map_err(|_| (4, "SinceTimestamp is invalid".to_owned()))?,
                    ),
                    None => None,
                };
                let messages = state
                    .pod_channels
                    .read()
                    .await
                    .list(pod_id, channel_id, since);
                serde_json::to_vec(&messages).map_err(|_| (1, "Pod response failed".to_owned()))
            }
            _ => Err((3, "Unknown method".to_owned())),
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
        drop(tunnels);
        tokio::spawn(async move {
            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
            while let Ok(read) = reader.read(&mut buffer).await {
                if read == 0 || incoming_tx.send(buffer[..read].to_vec()).await.is_err() {
                    break;
                }
            }
        });
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

#[derive(Debug, serde::Deserialize)]
struct ShadowQueryRequest {
    #[serde(alias = "MBID", alias = "mbid")]
    mbid: String,
}

#[derive(Debug, serde::Deserialize)]
struct ShadowBatchRequest {
    #[serde(alias = "MBIDs", alias = "mbids")]
    mbids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MeshContentRequest {
    #[serde(alias = "ContentId", alias = "contentId")]
    content_id: String,
    #[serde(default, alias = "Range", alias = "range")]
    range: Option<MeshContentRange>,
}

#[derive(Debug, serde::Deserialize)]
struct MeshContentRange {
    #[serde(alias = "Offset", alias = "offset")]
    offset: i64,
    #[serde(alias = "Length", alias = "length")]
    length: i64,
}

fn valid_shadow_mbid(value: &str) -> Result<&str, (i32, String)> {
    let value = value.trim();
    if !(8..=MAX_SHADOW_MBID_BYTES).contains(&value.len())
        || value.contains("..")
        || value.contains(['/', '\\'])
        || value.chars().any(char::is_control)
    {
        return Err((4, "Invalid MBID".to_owned()));
    }
    Ok(value)
}

async fn shadow_index_result(state: &super::AppState, mbid: &str) -> Option<serde_json::Value> {
    let discovery = state.content_discovery.read().await;
    let shadow = discovery
        .shadow_records()
        .iter()
        .find(|record| record.recording_id.eq_ignore_ascii_case(mbid))?;
    let canonical_variants = discovery
        .hash_entries()
        .iter()
        .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(mbid))
        .take(10)
        .filter_map(|entry| {
            let hash = [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|hash| !hash.is_empty())?;
            let hash = hex::decode(hash).ok()?;
            Some(serde_json::json!({
                "Codec": "FLAC",
                "BitrateKbps": 0,
                "SizeBytes": entry.size,
                "HashPrefix": BASE64.encode(&hash[..hash.len().min(16)]),
                "QualityScore": 1.0,
            }))
        })
        .collect::<Vec<_>>();
    let last_updated = chrono::DateTime::from_timestamp(shadow.updated_at as i64, 0)
        .map(|timestamp| timestamp.to_rfc3339());
    Some(serde_json::json!({
        "MBID": shadow.recording_id,
        "PeerCount": shadow.peer_ids.len(),
        "CanonicalVariants": canonical_variants,
        "LastUpdated": last_updated,
    }))
}

fn mesh_content_range(
    requested: Option<&MeshContentRange>,
    size: u64,
) -> Result<(u64, usize), (i32, String)> {
    let (offset, requested_length) = match requested {
        Some(range) if range.offset >= 0 && range.length >= 0 => {
            (range.offset as u64, range.length as u64)
        }
        Some(_) => return Err((4, "Invalid range request".to_owned())),
        None => (0, size),
    };
    if offset >= size {
        return Err((4, "Invalid range request".to_owned()));
    }
    let remaining = size - offset;
    let length = if requested_length == 0 {
        remaining
    } else {
        requested_length.min(remaining)
    };
    if length == 0 {
        return Err((4, "Invalid range request".to_owned()));
    }
    if length > MAX_MESH_CONTENT_BYTES as u64 {
        return Err((9, "Range too large; request a smaller range".to_owned()));
    }
    Ok((offset, length as usize))
}

#[derive(Debug, serde::Deserialize)]
struct PodIdRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct PodMessageRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
    #[serde(alias = "ChannelId")]
    channel_id: String,
    #[serde(alias = "Body")]
    body: String,
    #[serde(default, alias = "Signature")]
    signature: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PodMessagesRequest {
    #[serde(alias = "PodId")]
    pod_id: String,
    #[serde(alias = "ChannelId")]
    channel_id: String,
    #[serde(default, alias = "SinceTimestamp")]
    since_timestamp: Option<i64>,
}

fn bounded_required<'a>(
    value: &'a str,
    maximum: usize,
    name: &str,
) -> Result<&'a str, (i32, String)> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum {
        return Err((4, format!("{name} is invalid")));
    }
    Ok(value)
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
    let certificate = read_identity_file(
        &certificate_path,
        "certificate",
        MAX_CERTIFICATE_BYTES,
        false,
    )?;
    let private_key = read_identity_file(
        &private_key_path,
        "private key",
        MAX_PRIVATE_KEY_BYTES,
        true,
    )?;
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
    write_new_identity(
        &certificate_path,
        &private_key_path,
        &certificate,
        &private_key,
    )?;
    Ok((
        CertificateDer::from(certificate),
        PrivatePkcs8KeyDer::from(private_key),
    ))
}

fn write_new_identity(
    certificate_path: &Path,
    private_key_path: &Path,
    certificate: &[u8],
    private_key: &[u8],
) -> Result<(), String> {
    write_secret(certificate_path, certificate)?;
    if let Err(error) = write_secret(private_key_path, private_key) {
        return match fs::remove_file(certificate_path) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; overlay certificate rollback failed: {cleanup_error}"
            )),
        };
    }
    Ok(())
}

fn read_identity_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
    require_private_permissions: bool,
) -> Result<Option<Vec<u8>>, String> {
    #[cfg(not(unix))]
    let _ = require_private_permissions;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("overlay {label} metadata failed: {error}")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| format!("overlay {label} metadata failed: {error}"))?;
    if !opened_metadata.is_file() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    #[cfg(unix)]
    if require_private_permissions {
        use std::os::unix::fs::PermissionsExt;
        if opened_metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "overlay {label} must not be accessible by group or other users"
            ));
        }
    }
    if opened_metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut bytes = Vec::new();
    std::io::Read::take(&mut file, max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    if bytes.is_empty() {
        return Err(format!("overlay {label} is empty"));
    }
    Ok(Some(bytes))
}

fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_CLOEXEC);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("overlay identity creation failed: {error}"))?;
    if let Err(error) = std::io::Write::write_all(&mut file, bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity write failed: {error}"));
    }
    drop(file);
    if let Err(error) = fs::hard_link(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity publish failed: {error}"));
    }
    if let Err(error) = fs::remove_file(&temporary) {
        return Err(format!(
            "overlay identity published but temporary cleanup failed: {error}"
        ));
    }
    Ok(())
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
    fn overlay_keepalive_and_control_validation_match_the_frozen_lifecycle() {
        assert_eq!(OVERLAY_MESSAGE_READ_TIMEOUT, Duration::from_secs(30));
        assert_eq!(OVERLAY_KEEPALIVE_INTERVAL, Duration::from_secs(120));
        assert_eq!(OVERLAY_IDLE_TIMEOUT, Duration::from_secs(300));

        let now = i64::try_from(super::super::unix_timestamp_millis()).unwrap();
        assert!(Ping {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "ping".to_owned(),
            version: OVERLAY_VERSION,
            timestamp: now,
        }
        .validate()
        .is_ok());

        let mut liveness = OverlayLiveness {
            last_inbound: Instant::now() - OVERLAY_IDLE_TIMEOUT,
            last_ping: Instant::now() - OVERLAY_KEEPALIVE_INTERVAL,
        };
        assert!(liveness.is_idle());
        liveness.record_ping();
        assert!(liveness.is_idle(), "outbound pings are not peer activity");
        liveness.record_inbound();
        assert!(!liveness.is_idle());
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

    #[test]
    fn oversized_gateway_identity_is_rejected_before_parsing() {
        let root = temporary_directory("gateway-oversized-identity");
        fs::write(
            root.join("overlay-certificate.der"),
            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
        )
        .unwrap();
        fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
        let error = load_or_create_certificate(&root).unwrap_err();
        assert!(error.contains("certificate is too large"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_gateway_identity_is_rejected() {
        use std::os::unix::fs::symlink;

        let root = temporary_directory("gateway-symlinked-identity");
        let certificate_target = root.join("certificate-target.der");
        fs::write(&certificate_target, [1_u8]).unwrap();
        symlink(&certificate_target, root.join("overlay-certificate.der")).unwrap();
        fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
        let error = load_or_create_certificate(&root).unwrap_err();
        assert!(error.contains("certificate must be a regular file"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn exposed_gateway_private_key_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_directory("gateway-exposed-private-key");
        let path = root.join("overlay-private-key.der");
        fs::write(&path, [1_u8]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        let error = read_identity_file(&path, "private key", MAX_PRIVATE_KEY_BYTES, true)
            .expect_err("reject exposed private key");
        assert!(error.contains("must not be accessible by group or other users"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_identity_publish_removes_temporary_secret() {
        let root = temporary_directory("gateway-failed-secret-publish");
        let destination = root.join("overlay-private-key.der");
        fs::create_dir(&destination).unwrap();

        let error = write_secret(&destination, b"private-key").unwrap_err();
        assert!(error.contains("publish failed"));
        let names = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(names, vec![destination.file_name().unwrap()]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_private_key_publish_rolls_back_new_certificate() {
        let root = temporary_directory("gateway-identity-rollback");
        let certificate = root.join("overlay-certificate.der");
        let private_key = root.join("overlay-private-key.der");
        fs::create_dir(&private_key).unwrap();

        let error = write_new_identity(&certificate, &private_key, b"certificate", b"private-key")
            .unwrap_err();
        assert!(error.contains("publish failed"));
        assert!(!certificate.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn secret_publish_never_replaces_existing_identity() {
        let root = temporary_directory("gateway-existing-identity");
        let path = root.join("overlay-private-key.der");
        fs::write(&path, b"existing-key").unwrap();

        let error = write_secret(&path, b"replacement-key").unwrap_err();

        assert!(error.contains("publish failed"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), b"existing-key");
        let temporary_files = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "overlay-private-key.der")
            .count();
        assert_eq!(temporary_files, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
