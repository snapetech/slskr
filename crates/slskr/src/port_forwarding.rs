use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use ed25519_dalek::SigningKey;
use futures_util::{stream::FuturesUnordered, StreamExt};
use serde::Serialize;
use slskr_client::overlay::{
    connect_tls_overlay, CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshServiceCall,
    OpenTunnelRequest, OpenTunnelResponse, TlsOverlayClient, TunnelDataRequest, TunnelDataResponse,
    FEATURE_MESH_SERVICE,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, Mutex, OwnedSemaphorePermit, RwLock, Semaphore},
    task::JoinHandle,
    time::{sleep, timeout},
};

const MAX_FORWARDING_RULES: usize = 128;
const MAX_FORWARDING_CONNECTIONS: usize = 128;
pub(crate) const MAX_GATEWAY_ENDPOINTS: usize = 4;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;
const SERVICE_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const EMPTY_POLL_DELAY: Duration = Duration::from_millis(10);

#[derive(Clone, Debug)]
pub struct StartRequest {
    pub local_port: u16,
    pub pod_id: String,
    pub destination_host: String,
    pub destination_port: u16,
    pub service_name: Option<String>,
    pub gateway_username: String,
    pub gateway_endpoints: Vec<SocketAddr>,
    pub gateway_certificate_sha256: [u8; 32],
    pub local_username: String,
    pub authentication_key: Arc<SigningKey>,
}

#[derive(Debug)]
pub struct Manager {
    rules: RwLock<BTreeMap<u16, Arc<Rule>>>,
    connection_permits: Arc<Semaphore>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            rules: RwLock::new(BTreeMap::new()),
            connection_permits: Arc::new(Semaphore::new(MAX_FORWARDING_CONNECTIONS)),
        }
    }
}

impl Manager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, request: StartRequest) -> Result<Status, String> {
        validate_start_request(&request)?;
        let mut rules = self.rules.write().await;
        if rules.contains_key(&request.local_port) {
            return Err(format!(
                "Port {} is already being forwarded",
                request.local_port
            ));
        }
        if rules.len() >= MAX_FORWARDING_RULES {
            return Err("Port forwarding rule capacity is full".to_owned());
        }
        let listener = TcpListener::bind(("127.0.0.1", request.local_port))
            .await
            .map_err(|error| format!("Local forwarding listener bind failed: {error}"))?;
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let rule = Arc::new(Rule {
            request,
            active_connections: AtomicUsize::new(0),
            bytes_forwarded: Arc::new(AtomicU64::new(0)),
            cancel_tx,
            connection_permits: Arc::clone(&self.connection_permits),
            listener_task: Mutex::new(None),
            last_error: Mutex::new(None),
        });
        let task_rule = Arc::clone(&rule);
        let task = tokio::spawn(async move {
            task_rule.run(listener, cancel_rx).await;
        });
        *rule.listener_task.lock().await = Some(task);
        let status = rule.status();
        rules.insert(rule.request.local_port, rule);
        Ok(status)
    }

    pub async fn stop(&self, local_port: u16) -> bool {
        let rule = self.rules.write().await.remove(&local_port);
        let Some(rule) = rule else {
            return false;
        };
        let _ = rule.cancel_tx.send(true);
        if let Some(task) = rule.listener_task.lock().await.take() {
            let _ = timeout(Duration::from_secs(5), task).await;
        }
        true
    }

    pub async fn statuses(&self) -> Vec<Status> {
        self.rules
            .read()
            .await
            .values()
            .map(|rule| rule.status())
            .collect()
    }

    pub async fn status(&self, local_port: u16) -> Option<Status> {
        self.rules
            .read()
            .await
            .get(&local_port)
            .map(|rule| rule.status())
    }

    pub async fn used_ports(&self) -> Vec<u16> {
        self.rules.read().await.keys().copied().collect()
    }
}

#[derive(Debug)]
struct Rule {
    request: StartRequest,
    active_connections: AtomicUsize,
    bytes_forwarded: Arc<AtomicU64>,
    cancel_tx: watch::Sender<bool>,
    connection_permits: Arc<Semaphore>,
    listener_task: Mutex<Option<JoinHandle<()>>>,
    last_error: Mutex<Option<String>>,
}

impl Rule {
    async fn run(self: Arc<Self>, listener: TcpListener, mut cancel: watch::Receiver<bool>) {
        loop {
            tokio::select! {
                changed = cancel.changed() => {
                    if changed.is_err() || *cancel.borrow() {
                        break;
                    }
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _)) => {
                            let Ok(connection_permit) = Arc::clone(&self.connection_permits)
                                .try_acquire_owned()
                            else {
                                *self.last_error.lock().await = Some(
                                    "Port forwarding connection capacity is full".to_owned(),
                                );
                                continue;
                            };
                            let rule = Arc::clone(&self);
                            let connection_cancel = cancel.clone();
                            tokio::spawn(async move {
                                rule.handle_connection(
                                    stream,
                                    connection_cancel,
                                    connection_permit,
                                )
                                .await;
                            });
                        }
                        Err(error) => {
                            *self.last_error.lock().await = Some(format!("Local forwarding accept failed: {error}"));
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_connection(
        self: Arc<Self>,
        local: TcpStream,
        cancel: watch::Receiver<bool>,
        _connection_permit: OwnedSemaphorePermit,
    ) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        let result = self.forward_connection(local, cancel).await;
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        if let Err(error) = result {
            *self.last_error.lock().await = Some(error);
        }
    }

    async fn forward_connection(
        &self,
        local: TcpStream,
        mut cancel: watch::Receiver<bool>,
    ) -> Result<(), String> {
        let mut hello = MeshHello::new(
            &self.request.local_username,
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            uuid::Uuid::new_v4().simple().to_string(),
        )
        .map_err(|error| format!("Overlay hello failed: {error}"))?;
        hello
            .authenticate(
                &self.request.authentication_key,
                &self.request.gateway_certificate_sha256,
            )
            .map_err(|error| format!("Overlay hello authentication failed: {error}"))?;
        let connect = async {
            let mut connected = None;
            let mut last_error = "No gateway overlay endpoints are available".to_owned();
            let mut attempts = FuturesUnordered::new();
            for endpoint in &self.request.gateway_endpoints {
                attempts.push(connect_tls_overlay(
                    *endpoint,
                    self.request.gateway_certificate_sha256,
                    hello.clone(),
                ));
            }
            while let Some(result) = attempts.next().await {
                match result {
                    Ok(client)
                        if client
                            .remote_username
                            .eq_ignore_ascii_case(&self.request.gateway_username) =>
                    {
                        connected = Some(client);
                        break;
                    }
                    Ok(_) => {
                        last_error =
                            "Gateway overlay identity did not match the discovered peer".to_owned();
                    }
                    Err(error) => {
                        last_error = format!("Gateway overlay connection failed: {error}");
                    }
                }
            }
            connected.ok_or(last_error)
        };
        let client = tokio::select! {
            _ = cancel.changed() => return Ok(()),
            result = connect => result?,
        };
        let client = Arc::new(Mutex::new(client));
        let tunnel_id = tokio::select! {
            _ = cancel.changed() => return Ok(()),
            result = open_tunnel(&client, &self.request) => result?,
        };
        let (mut local_read, mut local_write) = local.into_split();
        let send_client = Arc::clone(&client);
        let send_tunnel = tunnel_id.clone();
        let send_bytes = Arc::clone(&self.bytes_forwarded);
        let mut send = tokio::spawn(async move {
            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
            loop {
                let read = local_read
                    .read(&mut buffer)
                    .await
                    .map_err(|error| format!("Local forwarding read failed: {error}"))?;
                if read == 0 {
                    return Ok::<(), String>(());
                }
                send_tunnel_data(&send_client, &send_tunnel, &buffer[..read]).await?;
                send_bytes.fetch_add(read as u64, Ordering::Relaxed);
            }
        });
        let receive_client = Arc::clone(&client);
        let receive_tunnel = tunnel_id.clone();
        let receive_bytes = Arc::clone(&self.bytes_forwarded);
        let mut receive = tokio::spawn(async move {
            loop {
                let data = receive_tunnel_data(&receive_client, &receive_tunnel).await?;
                if data.is_empty() {
                    sleep(EMPTY_POLL_DELAY).await;
                    continue;
                }
                local_write
                    .write_all(&data)
                    .await
                    .map_err(|error| format!("Local forwarding write failed: {error}"))?;
                receive_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
            }
            #[allow(unreachable_code)]
            Ok::<(), String>(())
        });
        let result = tokio::select! {
            changed = cancel.changed() => {
                let _ = changed;
                Ok(())
            }
            result = &mut send => result.map_err(|error| format!("Tunnel send task failed: {error}"))?,
            result = &mut receive => result.map_err(|error| format!("Tunnel receive task failed: {error}"))?,
        };
        send.abort();
        receive.abort();
        let _ = close_tunnel(&client, &tunnel_id).await;
        result?;
        Ok(())
    }

    fn status(&self) -> Status {
        let active_connections = self.active_connections.load(Ordering::Relaxed);
        let bytes_forwarded = self.bytes_forwarded.load(Ordering::Relaxed);
        Status {
            local_port: self.request.local_port,
            pod_id: self.request.pod_id.clone(),
            destination_host: self.request.destination_host.clone(),
            destination_port: self.request.destination_port,
            service_name: self.request.service_name.clone(),
            is_active: !*self.cancel_tx.borrow(),
            active_connections,
            bytes_forwarded,
            stream_mapping_enabled: true,
            stream_stats: None,
            performance: Performance {
                active_connections,
                total_bytes_transferred: bytes_forwarded,
            },
        }
    }
}

type GatewayClient = TlsOverlayClient;

async fn open_tunnel(
    client: &Arc<Mutex<GatewayClient>>,
    request: &StartRequest,
) -> Result<String, String> {
    let payload = serde_json::to_vec(
        &OpenTunnelRequest::new(
            &request.pod_id,
            &request.destination_host,
            request.destination_port,
            request.service_name.clone(),
            uuid::Uuid::new_v4().simple().to_string(),
        )
        .map_err(|error| format!("OpenTunnel request failed: {error}"))?,
    )
    .map_err(|error| format!("OpenTunnel payload failed: {error}"))?;
    let reply = service_call(client, "OpenTunnel", payload).await?;
    let response: OpenTunnelResponse = serde_json::from_slice(&reply)
        .map_err(|error| format!("OpenTunnel response failed: {error}"))?;
    if !response.accepted || response.tunnel_id.trim().is_empty() {
        return Err("Gateway rejected the tunnel".to_owned());
    }
    Ok(response.tunnel_id)
}

async fn send_tunnel_data(
    client: &Arc<Mutex<GatewayClient>>,
    tunnel_id: &str,
    data: &[u8],
) -> Result<(), String> {
    let payload = serde_json::to_vec(&TunnelDataRequest {
        tunnel_id: tunnel_id.to_owned(),
        data: data.to_vec(),
    })
    .map_err(|error| format!("TunnelData payload failed: {error}"))?;
    service_call(client, "TunnelData", payload).await?;
    Ok(())
}

async fn receive_tunnel_data(
    client: &Arc<Mutex<GatewayClient>>,
    tunnel_id: &str,
) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_vec(&GetTunnelDataRequest {
        tunnel_id: tunnel_id.to_owned(),
    })
    .map_err(|error| format!("GetTunnelData payload failed: {error}"))?;
    let reply = service_call(client, "GetTunnelData", payload).await?;
    let response: TunnelDataResponse = serde_json::from_slice(&reply)
        .map_err(|error| format!("GetTunnelData response failed: {error}"))?;
    if response.bytes_received != response.data.len() {
        return Err("GetTunnelData byte count did not match payload".to_owned());
    }
    Ok(response.data)
}

async fn close_tunnel(client: &Arc<Mutex<GatewayClient>>, tunnel_id: &str) -> Result<(), String> {
    let payload = serde_json::to_vec(&CloseTunnelRequest {
        tunnel_id: tunnel_id.to_owned(),
    })
    .map_err(|error| format!("CloseTunnel payload failed: {error}"))?;
    service_call(client, "CloseTunnel", payload).await?;
    Ok(())
}

async fn service_call(
    client: &Arc<Mutex<GatewayClient>>,
    method: &'static str,
    payload: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let call = MeshServiceCall::new(
        uuid::Uuid::new_v4().to_string(),
        "private-gateway",
        method,
        payload,
    )
    .map_err(|error| format!("{method} call failed: {error}"))?;
    let reply = timeout(SERVICE_CALL_TIMEOUT, async {
        client.lock().await.call(&call).await
    })
    .await
    .map_err(|_| format!("{method} call timed out"))?
    .map_err(|error| format!("{method} call failed: {error}"))?;
    if reply.status_code != 0 {
        return Err(format!(
            "{method} failed with status {}: {}",
            reply.status_code,
            reply.error_message.as_deref().unwrap_or("gateway error")
        ));
    }
    Ok(reply.payload)
}

fn validate_start_request(request: &StartRequest) -> Result<(), String> {
    if request.local_port < 1_024
        || request.destination_port == 0
        || request.pod_id.trim().is_empty()
        || request.destination_host.trim().is_empty()
        || request.gateway_username.trim().is_empty()
        || request.local_username.trim().is_empty()
        || request.gateway_endpoints.is_empty()
        || request.gateway_endpoints.len() > MAX_GATEWAY_ENDPOINTS
        || request.gateway_endpoints.iter().any(|endpoint| {
            endpoint.port() == 0
                || endpoint.ip().is_unspecified()
                || endpoint.ip().is_multicast()
                || matches!(endpoint.ip(), IpAddr::V4(ip) if ip.is_broadcast())
        })
    {
        return Err("Port forwarding request is invalid".to_owned());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub local_port: u16,
    pub pod_id: String,
    pub destination_host: String,
    pub destination_port: u16,
    pub service_name: Option<String>,
    pub is_active: bool,
    pub active_connections: usize,
    pub bytes_forwarded: u64,
    pub stream_mapping_enabled: bool,
    pub stream_stats: Option<serde_json::Value>,
    pub performance: Performance,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Performance {
    pub active_connections: usize,
    pub total_bytes_transferred: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::generate_simple_self_signed;
    use sha2::{Digest, Sha256};
    use slskr_client::overlay::{
        MeshHelloAck, MeshServiceReply, OverlayFramer, OVERLAY_MAGIC, OVERLAY_VERSION,
    };
    use tokio_rustls::{
        rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig},
        TlsAcceptor,
    };

    fn request(port: u16) -> StartRequest {
        StartRequest {
            local_port: port,
            pod_id: "pod:test".to_owned(),
            destination_host: "service".to_owned(),
            destination_port: 80,
            service_name: None,
            gateway_username: "gateway".to_owned(),
            gateway_endpoints: vec!["127.0.0.1:50305".parse().unwrap()],
            gateway_certificate_sha256: [7; 32],
            local_username: "local".to_owned(),
            authentication_key: Arc::new(SigningKey::from_bytes(&[9; 32])),
        }
    }

    #[tokio::test]
    async fn manager_binds_reports_and_stops_local_listener() {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let manager = Manager::new();

        let status = manager.start(request(port)).await.unwrap();
        assert_eq!(status.local_port, port);
        assert!(status.is_active);
        assert!(TcpStream::connect(("127.0.0.1", port)).await.is_ok());
        assert_eq!(manager.statuses().await.len(), 1);
        assert!(manager.stop(port).await);
        assert!(!manager.stop(port).await);
        assert!(manager.statuses().await.is_empty());
    }

    #[tokio::test]
    async fn manager_rejects_duplicate_and_occupied_ports() {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        let manager = Manager::new();
        assert!(manager.start(request(port)).await.is_err());
        drop(probe);
        manager.start(request(port)).await.unwrap();
        assert!(manager.start(request(port)).await.is_err());
        assert!(manager.stop(port).await);
    }

    #[tokio::test]
    async fn stopping_rule_cancels_stalled_gateway_handshake_and_releases_permit() {
        let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_endpoint = gateway_listener.local_addr().unwrap();
        let stalled_gateway = tokio::spawn(async move {
            let (_stream, _) = gateway_listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let local_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_port = local_probe.local_addr().unwrap().port();
        drop(local_probe);
        let manager = Manager::new();
        let mut request = request(local_port);
        request.gateway_endpoints = vec![gateway_endpoint];
        manager.start(request).await.unwrap();
        let rule = Arc::clone(manager.rules.read().await.get(&local_port).unwrap());
        let local = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();

        timeout(Duration::from_secs(2), async {
            while rule.active_connections.load(Ordering::Relaxed) == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            manager.connection_permits.available_permits(),
            MAX_FORWARDING_CONNECTIONS - 1
        );

        assert!(manager.stop(local_port).await);
        timeout(Duration::from_secs(2), async {
            while rule.active_connections.load(Ordering::Relaxed) != 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("stalled gateway setup should be cancelled promptly");
        assert_eq!(
            manager.connection_permits.available_permits(),
            MAX_FORWARDING_CONNECTIONS
        );
        drop(local);
        stalled_gateway.abort();
    }

    #[tokio::test]
    async fn manager_forwards_bytes_through_tls_private_gateway_calls() {
        let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let certificate_sha256 = Sha256::digest(certificate.as_ref()).into();
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        let config =
            ServerConfig::builder_with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key.into())
                .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_endpoint = gateway_listener.local_addr().unwrap();
        let gateway = tokio::spawn(async move {
            let (tcp, _) = gateway_listener.accept().await.unwrap();
            let tls = acceptor.accept(tcp).await.unwrap();
            let mut framer = OverlayFramer::new(tls);
            let hello: slskr_client::overlay::MeshHello = framer.read().await.unwrap();
            framer
                .write(&MeshHelloAck {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_hello_ack".to_owned(),
                    version: OVERLAY_VERSION,
                    username: "gateway".to_owned(),
                    features: vec![FEATURE_MESH_SERVICE.to_owned()],
                    soulseek_ports: None,
                    overlay_port: Some(gateway_endpoint.port()),
                    nonce_echo: hello.nonce,
                })
                .await
                .unwrap();
            let mut buffered = Vec::new();
            loop {
                let call: MeshServiceCall = framer.read().await.unwrap();
                let (status_code, payload, done) = match call.method.as_str() {
                    "OpenTunnel" => {
                        let request: OpenTunnelRequest =
                            serde_json::from_slice(&call.payload).unwrap();
                        assert_eq!(request.pod_id, "pod:test");
                        (
                            0,
                            serde_json::to_vec(&OpenTunnelResponse {
                                tunnel_id: "tunnel-1".to_owned(),
                                accepted: true,
                            })
                            .unwrap(),
                            false,
                        )
                    }
                    "TunnelData" => {
                        let request: TunnelDataRequest =
                            serde_json::from_slice(&call.payload).unwrap();
                        buffered.extend_from_slice(&request.data);
                        (0, br#"{"Sent":5}"#.to_vec(), false)
                    }
                    "GetTunnelData" => {
                        let data = std::mem::take(&mut buffered);
                        (
                            0,
                            serde_json::to_vec(&TunnelDataResponse {
                                bytes_received: data.len(),
                                data,
                            })
                            .unwrap(),
                            false,
                        )
                    }
                    "CloseTunnel" => (0, br#"{"Closed":true}"#.to_vec(), true),
                    other => panic!("unexpected gateway method {other}"),
                };
                framer
                    .write(&MeshServiceReply {
                        magic: OVERLAY_MAGIC.to_owned(),
                        message_type: "mesh_service_reply".to_owned(),
                        version: OVERLAY_VERSION,
                        correlation_id: call.correlation_id,
                        status_code,
                        payload,
                        error_message: None,
                    })
                    .await
                    .unwrap();
                if done {
                    break;
                }
            }
        });

        let local_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_port = local_probe.local_addr().unwrap().port();
        drop(local_probe);
        let unavailable_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let unavailable_endpoint = unavailable_probe.local_addr().unwrap();
        drop(unavailable_probe);
        let manager = Manager::new();
        let mut request = request(local_port);
        request.gateway_endpoints = vec![unavailable_endpoint, gateway_endpoint];
        request.gateway_certificate_sha256 = certificate_sha256;
        manager.start(request).await.unwrap();
        let mut local = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();
        local.write_all(b"hello").await.unwrap();
        let mut echoed = [0_u8; 5];
        timeout(Duration::from_secs(5), local.read_exact(&mut echoed))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&echoed, b"hello");
        timeout(Duration::from_secs(2), async {
            loop {
                let status = manager.status(local_port).await.unwrap();
                if status.bytes_forwarded == 10 {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("forwarding totals should update while the connection remains active");
        assert!(manager.stop(local_port).await);
        timeout(Duration::from_secs(5), gateway)
            .await
            .unwrap()
            .unwrap();
    }
}
