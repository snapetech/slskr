use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use ed25519_dalek::SigningKey;
use futures_util::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use slskr_client::overlay::{
    connect_tls_overlay, CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshServiceCall,
    OpenTunnelRequest, OpenTunnelResponse, TlsOverlayClient, TunnelDataRequest, TunnelDataResponse,
    FEATURE_MESH_SERVICE,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, Mutex, OwnedSemaphorePermit, RwLock, Semaphore},
    task::{JoinHandle, JoinSet},
    time::{sleep, timeout},
};

pub(crate) const MAX_FORWARDING_RULES: usize = 128;
pub(crate) const MAX_FORWARDING_CONNECTIONS: usize = 128;
pub(crate) const MAX_GATEWAY_ENDPOINTS: usize = 4;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;
const MAX_TUNNEL_ID_BYTES: usize = 128;
const SERVICE_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const TUNNEL_CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
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
    closed: AtomicBool,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            rules: RwLock::new(BTreeMap::new()),
            connection_permits: Arc::new(Semaphore::new(MAX_FORWARDING_CONNECTIONS)),
            closed: AtomicBool::new(false),
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        for rule in self.rules.get_mut().values() {
            let _ = rule.cancel_tx.send(true);
            if let Ok(mut task) = rule.listener_task.try_lock() {
                if let Some(task) = task.as_mut() {
                    task.0.abort();
                }
            }
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
        if self.closed.load(Ordering::Acquire) {
            return Err("Port forwarding manager is shut down".to_owned());
        }
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
        let started_at_ms = crate::utils::unix_timestamp_millis();
        let rule = Arc::new(Rule {
            request,
            active_connections: AtomicUsize::new(0),
            bytes_in: Arc::new(AtomicU64::new(0)),
            bytes_out: Arc::new(AtomicU64::new(0)),
            bytes_forwarded: Arc::new(AtomicU64::new(0)),
            cancel_tx,
            connection_permits: Arc::clone(&self.connection_permits),
            listener_task: Mutex::new(None),
            last_error: Mutex::new(None),
            started_at_ms,
            last_activity_ms: Arc::new(AtomicU64::new(started_at_ms)),
        });
        let task_rule = Arc::clone(&rule);
        let task = tokio::spawn(async move {
            task_rule.run(listener, cancel_rx).await;
        });
        *rule.listener_task.lock().await = Some(ListenerTask(task));
        let status = rule.status();
        rules.insert(rule.request.local_port, rule);
        Ok(status)
    }

    pub async fn stop(&self, local_port: u16) -> bool {
        let rule = self.rules.write().await.remove(&local_port);
        let Some(rule) = rule else {
            return false;
        };
        stop_rule(rule).await;
        true
    }

    pub(crate) async fn shutdown(&self) {
        let rules = {
            let mut rules = self.rules.write().await;
            self.closed.store(true, Ordering::Release);
            std::mem::take(&mut *rules)
        };
        for rule in rules.values() {
            let _ = rule.cancel_tx.send(true);
        }
        let mut stops = FuturesUnordered::new();
        for rule in rules.into_values() {
            stops.push(stop_rule(rule));
        }
        while stops.next().await.is_some() {}
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
struct ListenerTask(JoinHandle<()>);
impl Drop for ListenerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn stop_rule(rule: Arc<Rule>) {
    let _ = rule.cancel_tx.send(true);
    let task = rule.listener_task.lock().await.take();
    if let Some(mut task) = task {
        if timeout(Duration::from_secs(5), &mut task.0).await.is_err() {
            task.0.abort();
            let _ = (&mut task.0).await;
        }
    }
}

struct ActiveConnection<'a>(&'a AtomicUsize);
impl Drop for ActiveConnection<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct Rule {
    request: StartRequest,
    active_connections: AtomicUsize,
    bytes_in: Arc<AtomicU64>,
    bytes_out: Arc<AtomicU64>,
    bytes_forwarded: Arc<AtomicU64>,
    cancel_tx: watch::Sender<bool>,
    connection_permits: Arc<Semaphore>,
    listener_task: Mutex<Option<ListenerTask>>,
    last_error: Mutex<Option<String>>,
    started_at_ms: u64,
    last_activity_ms: Arc<AtomicU64>,
}

impl Rule {
    async fn run(self: Arc<Self>, listener: TcpListener, mut cancel: watch::Receiver<bool>) {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                changed = cancel.changed() => {
                    if changed.is_err() || *cancel.borrow() {
                        break;
                    }
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
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
                            while connections.try_join_next().is_some() {}
                            let rule = Arc::clone(&self);
                            let connection_cancel = cancel.clone();
                            connections.spawn(async move {
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
        let _ = self.cancel_tx.send(true);
        // Give cancelled connections time to close their remote tunnels;
        // forced parent cancellation still drops this set and aborts children.
        if timeout(Duration::from_secs(3), async {
            while connections.join_next().await.is_some() {}
        })
        .await
        .is_err()
        {
            connections.shutdown().await;
        }
    }

    async fn handle_connection(
        self: Arc<Self>,
        local: TcpStream,
        cancel: watch::Receiver<bool>,
        _connection_permit: OwnedSemaphorePermit,
    ) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        let _active = ActiveConnection(&self.active_connections);
        let result = self.forward_connection(local, cancel).await;
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
        let send_bytes_out = Arc::clone(&self.bytes_out);
        let send_last_activity = Arc::clone(&self.last_activity_ms);
        let result = {
            let send = async move {
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
                    send_bytes_out.fetch_add(read as u64, Ordering::Relaxed);
                    send_last_activity
                        .store(crate::utils::unix_timestamp_millis(), Ordering::Relaxed);
                }
            };
            let receive_client = Arc::clone(&client);
            let receive_tunnel = tunnel_id.clone();
            let receive_bytes = Arc::clone(&self.bytes_forwarded);
            let receive_bytes_in = Arc::clone(&self.bytes_in);
            let receive_last_activity = Arc::clone(&self.last_activity_ms);
            let receive = async move {
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
                    receive_bytes_in.fetch_add(data.len() as u64, Ordering::Relaxed);
                    receive_last_activity
                        .store(crate::utils::unix_timestamp_millis(), Ordering::Relaxed);
                }
                #[allow(unreachable_code)]
                Ok::<(), String>(())
            };
            tokio::pin!(send, receive);
            tokio::select! {
                changed = cancel.changed() => {
                    let _ = changed;
                    Ok(())
                }
                result = &mut send => result,
                result = &mut receive => result,
            }
        };
        let close_result =
            match timeout(TUNNEL_CLOSE_TIMEOUT, close_tunnel(&client, &tunnel_id)).await {
                Ok(result) => result,
                Err(_) => Err("Gateway tunnel close timed out".to_owned()),
            };
        match (result, close_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), Err(close_error)) => Err(format!(
                "{error}; gateway tunnel close failed: {close_error}"
            )),
        }
    }

    fn status(&self) -> Status {
        let active_connections = self.active_connections.load(Ordering::Relaxed);
        let bytes_in = self.bytes_in.load(Ordering::Relaxed);
        let bytes_out = self.bytes_out.load(Ordering::Relaxed);
        let bytes_forwarded = self.bytes_forwarded.load(Ordering::Relaxed);
        Status {
            local_port: self.request.local_port,
            pod_id: self.request.pod_id.clone(),
            destination_host: self.request.destination_host.clone(),
            destination_port: self.request.destination_port,
            service_name: self.request.service_name.clone(),
            is_active: !*self.cancel_tx.borrow(),
            active_connections,
            bytes_in,
            bytes_out,
            bytes_forwarded,
            started_at: self.started_at_ms,
            last_activity: self.last_activity_ms.load(Ordering::Relaxed),
            stream_mapping_enabled: true,
            stream_stats: None,
            performance: Performance::new(active_connections, bytes_forwarded),
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
    parse_open_tunnel_response(&reply)
}

fn parse_open_tunnel_response(reply: &[u8]) -> Result<String, String> {
    let response: OpenTunnelResponse = serde_json::from_slice(reply)
        .map_err(|error| format!("OpenTunnel response failed: {error}"))?;
    if !response.accepted
        || response.tunnel_id.trim().is_empty()
        || response.tunnel_id.len() > MAX_TUNNEL_ID_BYTES
    {
        return Err("Gateway rejected the tunnel".to_owned());
    }
    Ok(response.tunnel_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TunnelDataAcknowledgement {
    sent: usize,
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
    let reply = service_call(client, "TunnelData", payload).await?;
    validate_tunnel_data_acknowledgement(&reply, data.len())
}

fn validate_tunnel_data_acknowledgement(reply: &[u8], expected: usize) -> Result<(), String> {
    let acknowledgement: TunnelDataAcknowledgement = serde_json::from_slice(reply)
        .map_err(|error| format!("TunnelData response failed: {error}"))?;
    if acknowledgement.sent != expected {
        return Err("TunnelData byte count did not match payload".to_owned());
    }
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
    parse_tunnel_data_response(&reply)
}

fn parse_tunnel_data_response(reply: &[u8]) -> Result<Vec<u8>, String> {
    let response: TunnelDataResponse = serde_json::from_slice(reply)
        .map_err(|error| format!("GetTunnelData response failed: {error}"))?;
    if response.bytes_received != response.data.len() {
        return Err("GetTunnelData byte count did not match payload".to_owned());
    }
    if response.data.len() > TUNNEL_CHUNK_BYTES {
        return Err("GetTunnelData payload is too large".to_owned());
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

pub(crate) fn validate_start_request(request: &StartRequest) -> Result<(), String> {
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
    for username in [&request.local_username, &request.gateway_username] {
        MeshHello::new(username, Vec::new(), None, None, "validation")
            .map_err(|_| "Port forwarding username is invalid".to_owned())?;
    }
    OpenTunnelRequest::new(
        &request.pod_id,
        &request.destination_host,
        request.destination_port,
        request.service_name.clone(),
        "validation",
    )
    .map_err(|_| "Port forwarding destination is invalid".to_owned())?;
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
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub bytes_forwarded: u64,
    pub started_at: u64,
    pub last_activity: u64,
    pub stream_mapping_enabled: bool,
    pub stream_stats: Option<serde_json::Value>,
    pub performance: Performance,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Performance {
    pub active_connections: usize,
    pub total_bytes_transferred: u64,
    pub average_bytes_per_connection: u64,
    pub is_high_throughput: bool,
    pub efficiency_rating: f64,
}

impl Performance {
    /// Matches the oracle's real `PortForwardingPerformance` computed
    /// properties (`AverageBytesPerConnection`, `IsHighThroughput`,
    /// `EfficiencyRating`), derived from the same two raw counters.
    fn new(active_connections: usize, total_bytes_transferred: u64) -> Self {
        let average_bytes_per_connection = if active_connections > 0 {
            total_bytes_transferred / active_connections as u64
        } else {
            0
        };
        let is_high_throughput = total_bytes_transferred > 1024 * 1024;
        let efficiency_rating = if active_connections > 0 && total_bytes_transferred > 0 {
            total_bytes_transferred as f64 / (active_connections as f64 * 1000.0)
        } else {
            0.0
        };
        Self {
            active_connections,
            total_bytes_transferred,
            average_bytes_per_connection,
            is_high_throughput,
            efficiency_rating,
        }
    }
}

#[cfg(test)]
#[path = "port_forwarding_tests.rs"]
mod tests;
