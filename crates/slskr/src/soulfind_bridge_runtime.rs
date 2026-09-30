use super::*;

pub(super) const BRIDGE_LOGIN: i32 = 1;
pub(super) const BRIDGE_LOGIN_RESPONSE: i32 = 2;
pub(super) const BRIDGE_SEARCH_REQUEST: i32 = 3;
pub(super) const BRIDGE_SEARCH_RESPONSE: i32 = 4;
pub(super) const BRIDGE_DOWNLOAD_REQUEST: i32 = 5;
pub(super) const BRIDGE_DOWNLOAD_RESPONSE: i32 = 6;
pub(super) const BRIDGE_ROOM_LIST_REQUEST: i32 = 7;
pub(super) const BRIDGE_ROOM_LIST_RESPONSE: i32 = 8;
pub(super) const BRIDGE_MAX_FRAME_BYTES: usize = 1024 * 1024;
const BRIDGE_MAX_STRING_BYTES: usize = 1024 * 1024;
const BRIDGE_READ_TIMEOUT: Duration = Duration::from_secs(120);
const BRIDGE_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// The frozen native profile bridge uses a deliberately small Soulseek-compatible
/// frame: a little-endian length containing the four-byte message type and
/// payload, followed by the little-endian type and payload.  Keep this parser
/// bounded and independent of the HTTP bridge routes so legacy clients use
/// the same real protocol boundary as the target service.
pub(super) async fn bridge_read_frame(
    stream: &mut TcpStream,
) -> Result<Option<(i32, Vec<u8>)>, &'static str> {
    tokio::time::timeout(BRIDGE_READ_TIMEOUT, bridge_read_frame_inner(stream))
        .await
        .map_err(|_| "bridge read timed out")?
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn bridge_read_frame_with_timeout(
    stream: &mut TcpStream,
    timeout_duration: Duration,
) -> Result<Option<(i32, Vec<u8>)>, &'static str> {
    tokio::time::timeout(timeout_duration, bridge_read_frame_inner(stream))
        .await
        .map_err(|_| "bridge read timed out")?
}

async fn bridge_read_frame_inner(
    stream: &mut TcpStream,
) -> Result<Option<(i32, Vec<u8>)>, &'static str> {
    let mut length_bytes = [0_u8; 4];
    if stream.read_exact(&mut length_bytes).await.is_err() {
        return Ok(None);
    }
    let length = u32::from_le_bytes(length_bytes) as usize;
    if !(4..=BRIDGE_MAX_FRAME_BYTES).contains(&length) {
        return Err("invalid bridge message length");
    }
    let mut type_bytes = [0_u8; 4];
    stream
        .read_exact(&mut type_bytes)
        .await
        .map_err(|_| "incomplete bridge message type")?;
    let mut payload = vec![0_u8; length - 4];
    if !payload.is_empty() {
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|_| "incomplete bridge message payload")?;
    }
    Ok(Some((i32::from_le_bytes(type_bytes), payload)))
}

pub(super) async fn bridge_write_frame(
    stream: &mut TcpStream,
    message_type: i32,
    payload: &[u8],
) -> Result<(), &'static str> {
    tokio::time::timeout(
        BRIDGE_WRITE_TIMEOUT,
        bridge_write_frame_inner(stream, message_type, payload),
    )
    .await
    .map_err(|_| "bridge write timed out")?
}

async fn bridge_write_frame_inner(
    stream: &mut TcpStream,
    message_type: i32,
    payload: &[u8],
) -> Result<(), &'static str> {
    let length = 4_usize
        .checked_add(payload.len())
        .filter(|length| *length <= BRIDGE_MAX_FRAME_BYTES)
        .ok_or("bridge response is too large")?;
    let length = u32::try_from(length).map_err(|_| "bridge response length overflow")?;
    stream
        .write_all(&length.to_le_bytes())
        .await
        .map_err(|_| "could not write bridge response length")?;
    stream
        .write_all(&message_type.to_le_bytes())
        .await
        .map_err(|_| "could not write bridge response type")?;
    if !payload.is_empty() {
        stream
            .write_all(payload)
            .await
            .map_err(|_| "could not write bridge response payload")?;
    }
    stream
        .flush()
        .await
        .map_err(|_| "could not flush bridge response")
}

pub(super) fn bridge_read_i32(payload: &[u8], cursor: &mut usize) -> Option<i32> {
    let end = cursor.checked_add(4)?;
    let bytes = payload.get(*cursor..end)?;
    *cursor = end;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

pub(super) fn bridge_read_string(payload: &[u8], cursor: &mut usize) -> Option<String> {
    let length = bridge_read_i32(payload, cursor)?;
    if !(0..=i32::try_from(BRIDGE_MAX_STRING_BYTES).ok()?).contains(&length) {
        return None;
    }
    let length = usize::try_from(length).ok()?;
    let end = cursor.checked_add(length)?;
    let value = std::str::from_utf8(payload.get(*cursor..end)?)
        .ok()?
        .to_owned();
    *cursor = end;
    Some(value)
}

pub(super) fn bridge_write_i32(payload: &mut Vec<u8>, value: i32) {
    payload.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn bridge_write_string(payload: &mut Vec<u8>, value: &str) {
    let value = value.as_bytes();
    let value = &value[..value.len().min(BRIDGE_MAX_STRING_BYTES)];
    bridge_write_i32(payload, i32::try_from(value.len()).unwrap_or(i32::MAX));
    payload.extend_from_slice(value);
}

pub(super) fn bridge_login_response(success: bool, message: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(u8::from(success));
    bridge_write_string(&mut payload, message);
    payload
}

pub(super) fn bridge_download_wire_response(
    success: bool,
    message_or_id: &str,
    token: i32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(u8::from(success));
    bridge_write_string(&mut payload, message_or_id);
    bridge_write_i32(&mut payload, token);
    payload
}

fn bridge_password_matches(provided: &str, configured: &str) -> bool {
    let provided = provided.as_bytes();
    let configured = configured.as_bytes();
    let length = provided.len().max(configured.len());
    let mut provided_padded = vec![0_u8; length];
    let mut configured_padded = vec![0_u8; length];
    provided_padded[..provided.len()].copy_from_slice(provided);
    configured_padded[..configured.len()].copy_from_slice(configured);
    provided.len() == configured.len()
        && subtle_constant_time_equal(&provided_padded, &configured_padded)
}

fn subtle_constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

struct BridgeClientSession {
    username: String,
    requests_in_window: u32,
    request_window_started: Instant,
    transfer_count: u32,
    active_transfer_id: Option<String>,
}

impl BridgeClientSession {
    fn new() -> Self {
        Self {
            username: String::new(),
            requests_in_window: 0,
            request_window_started: Instant::now(),
            transfer_count: 0,
            active_transfer_id: None,
        }
    }

    fn consume_request_quota(&mut self, limit: u32) -> bool {
        if self.request_window_started.elapsed() >= Duration::from_secs(60) {
            self.request_window_started = Instant::now();
            self.requests_in_window = 0;
        }
        if self.requests_in_window >= limit.max(1) {
            return false;
        }
        self.requests_in_window = self.requests_in_window.saturating_add(1);
        true
    }
}

pub(super) fn spawn_bridge_server(state: Arc<AppState>) {
    if !state.config.media_services.virtual_soulfind.bridge.enabled {
        return;
    }
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        run_bridge_server(state).await;
    });
}

struct BridgeClientCompletion(Option<tokio::sync::oneshot::Sender<()>>);

impl Drop for BridgeClientCompletion {
    fn drop(&mut self) {
        if let Some(completion) = self.0.take() {
            let _ = completion.send(());
        }
    }
}

pub(super) struct BridgeClientTasks {
    stop: tokio::sync::watch::Sender<bool>,
    pending: Vec<(String, tokio::sync::oneshot::Receiver<()>)>,
}

impl BridgeClientTasks {
    pub(super) fn new() -> Self {
        let (stop, _) = tokio::sync::watch::channel(false);
        Self {
            stop,
            pending: Vec::new(),
        }
    }

    pub(super) fn spawn(
        &mut self,
        state: &Arc<AppState>,
        client_id: String,
        stream: TcpStream,
    ) -> bool {
        let mut stop = self.stop.subscribe();
        let (finished, completion) = tokio::sync::oneshot::channel();
        let completion_guard = BridgeClientCompletion(Some(finished));
        let task_state = Arc::clone(state);
        let task_id = client_id.clone();
        let admitted = state.managed_background_tasks.try_spawn(async move {
            let _completion = completion_guard;
            tokio::select! {
                biased;
                _ = async { let _ = stop.wait_for(|stopped| *stopped).await; } => {},
                _ = bridge_handle_client(task_id.clone(), stream, Arc::clone(&task_state)) => {},
            }
            bridge_remove_client(&task_state, &task_id).await;
        });
        if admitted {
            self.pending.push((client_id, completion));
        }
        admitted
    }

    async fn reap(&mut self, state: &AppState) {
        let mut finished = Vec::new();
        self.pending.retain_mut(|(client_id, completion)| {
            if matches!(
                completion.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty)
            ) {
                true
            } else {
                finished.push(client_id.clone());
                false
            }
        });
        for client_id in finished {
            bridge_remove_client(state, &client_id).await;
        }
    }

    pub(super) async fn shutdown(self, state: &AppState) {
        let _ = self.stop.send(true);
        let joined = time::timeout(MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, async {
            for (client_id, completion) in self.pending {
                let _ = completion.await;
                bridge_remove_client(state, &client_id).await;
            }
        })
        .await;
        if joined.is_err() {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "bridge",
                "Soulfind bridge client shutdown exceeded its deadline".to_owned(),
            )
            .await;
        }
    }
}

async fn run_bridge_server(state: Arc<AppState>) {
    let bridge = state.config.media_services.virtual_soulfind.bridge.clone();
    if !bridge.bind_address.is_loopback()
        && (!bridge.require_auth || bridge.password.trim().is_empty())
    {
        record_daemon_log(
            &state,
            logging::LogLevel::Error,
            "bridge",
            "refusing non-loopback Soulfind bridge without password authentication".to_owned(),
        )
        .await;
        return;
    }

    let address = SocketAddr::new(bridge.bind_address, bridge.port);
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            record_daemon_log(
                &state,
                logging::LogLevel::Error,
                "bridge",
                format!("could not bind Soulfind bridge at {address}: {error}"),
            )
            .await;
            return;
        }
    };
    mutate_runtime_compat_state_in_memory(&state, |runtime| {
        runtime.set_bridge_running(true, bridge.enabled);
        runtime.bridge_started_at = Some(unix_timestamp());
    })
    .await;
    record_daemon_log(
        &state,
        logging::LogLevel::Info,
        "bridge",
        format!("Soulfind bridge listening on {address}"),
    )
    .await;

    let mut clients = BridgeClientTasks::new();
    loop {
        clients.reap(&state).await;
        if !state.runtime.read().await.bridge_running {
            break;
        }
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            _ = time::sleep(Duration::from_millis(250)) => continue,
        };
        let (mut stream, remote) = match accepted {
            Ok(connection) => connection,
            Err(error) => {
                record_daemon_log(
                    &state,
                    logging::LogLevel::Error,
                    "bridge",
                    format!("Soulfind bridge listener failed at {address}: {error}"),
                )
                .await;
                break;
            }
        };
        let client_id = uuid::Uuid::new_v4().simple().to_string();
        let mut runtime = state.runtime.write().await;
        if runtime.bridge_active_clients.len() >= bridge.max_clients {
            drop(runtime);
            let _ = stream.shutdown().await;
            continue;
        }
        let now = unix_timestamp();
        runtime.bridge_total_connections = runtime.bridge_total_connections.saturating_add(1);
        runtime.bridge_active_clients.insert(
            client_id.clone(),
            serde_json::json!({
                "clientId": client_id,
                "clientType": "Unknown",
                "ipAddress": remote.ip().to_string(),
                "connectedAt": now,
                "requestCount": 0,
                "lastActivity": now,
            }),
        );
        drop(runtime);
        if !clients.spawn(&state, client_id.clone(), stream) {
            bridge_remove_client(&state, &client_id).await;
            break;
        }
    }
    clients.shutdown(&state).await;

    mutate_runtime_compat_state_in_memory(&state, |runtime| {
        runtime.set_bridge_running(false, bridge.enabled);
    })
    .await;
}

pub(super) async fn bridge_handle_client(
    client_id: String,
    mut stream: TcpStream,
    state: Arc<AppState>,
) {
    let bridge = state.config.media_services.virtual_soulfind.bridge.clone();
    let mut session = BridgeClientSession::new();

    let login = match bridge_read_frame(&mut stream).await {
        Ok(Some((message_type, payload))) if message_type == BRIDGE_LOGIN => {
            let mut cursor = 0;
            bridge_read_string(&payload, &mut cursor).zip(bridge_read_string(&payload, &mut cursor))
        }
        _ => None,
    };
    let Some((username, password)) = login else {
        let _ = bridge_write_frame(
            &mut stream,
            BRIDGE_LOGIN_RESPONSE,
            &bridge_login_response(false, "Login message required"),
        )
        .await;
        bridge_remove_client(&state, &client_id).await;
        return;
    };
    if bridge.require_auth && !bridge_password_matches(&password, &bridge.password) {
        let _ = bridge_write_frame(
            &mut stream,
            BRIDGE_LOGIN_RESPONSE,
            &bridge_login_response(false, "Invalid username or password"),
        )
        .await;
        bridge_remove_client(&state, &client_id).await;
        return;
    }
    session.username = username;
    {
        let mut runtime = state.runtime.write().await;
        if let Some(client) = runtime.bridge_active_clients.get_mut(&client_id) {
            let client_type = if session.username.trim().is_empty() {
                "Soulseek Legacy".to_owned()
            } else {
                format!("Soulseek Legacy ({})", session.username)
            };
            client["clientType"] = serde_json::json!(client_type);
            client["lastActivity"] = serde_json::json!(unix_timestamp());
        }
    }
    if bridge_write_frame(
        &mut stream,
        BRIDGE_LOGIN_RESPONSE,
        &bridge_login_response(true, "Login successful"),
    )
    .await
    .is_err()
    {
        bridge_remove_client(&state, &client_id).await;
        return;
    }

    loop {
        let frame = match bridge_read_frame(&mut stream).await {
            Ok(frame) => frame,
            Err(error) => {
                record_daemon_log(
                    &state,
                    logging::LogLevel::Warn,
                    "bridge",
                    format!("bridge client {client_id} read failed: {error}"),
                )
                .await;
                break;
            }
        };
        let Some((message_type, payload)) = frame else {
            break;
        };
        if !session.consume_request_quota(bridge.max_requests_per_minute) {
            let _ = bridge_write_frame(
                &mut stream,
                BRIDGE_LOGIN_RESPONSE,
                &bridge_login_response(false, "Request quota exceeded"),
            )
            .await;
            break;
        }
        bridge_record_request(&state, &client_id, message_type).await;
        let result = match message_type {
            BRIDGE_SEARCH_REQUEST => bridge_handle_search(&mut stream, payload, &state).await,
            BRIDGE_DOWNLOAD_REQUEST => {
                bridge_handle_download(&mut stream, payload, &state, &mut session).await
            }
            BRIDGE_ROOM_LIST_REQUEST => bridge_handle_room_list(&mut stream, &state).await,
            _ => {
                bridge_write_frame(
                    &mut stream,
                    BRIDGE_LOGIN_RESPONSE,
                    &bridge_login_response(false, "Unknown message type"),
                )
                .await
            }
        };
        if let Err(error) = result {
            record_daemon_log(
                &state,
                logging::LogLevel::Warn,
                "bridge",
                format!("bridge client {client_id} request failed: {error}"),
            )
            .await;
            break;
        }
    }
    bridge_remove_client(&state, &client_id).await;
}

async fn bridge_record_request(state: &AppState, client_id: &str, message_type: i32) {
    let mut runtime = state.runtime.write().await;
    if let Some(client) = runtime.bridge_active_clients.get_mut(client_id) {
        let request_count = client["requestCount"]
            .as_u64()
            .unwrap_or(0)
            .saturating_add(1);
        client["requestCount"] = serde_json::json!(request_count);
        client["lastActivity"] = serde_json::json!(unix_timestamp());
    }
    match message_type {
        BRIDGE_SEARCH_REQUEST => {
            runtime.bridge_total_searches = runtime.bridge_total_searches.saturating_add(1)
        }
        BRIDGE_DOWNLOAD_REQUEST => {
            runtime.bridge_total_downloads = runtime.bridge_total_downloads.saturating_add(1)
        }
        BRIDGE_ROOM_LIST_REQUEST => {
            runtime.bridge_total_room_joins = runtime.bridge_total_room_joins.saturating_add(1)
        }
        _ => {}
    }
}

async fn bridge_remove_client(state: &AppState, client_id: &str) {
    state
        .runtime
        .write()
        .await
        .bridge_active_clients
        .remove(client_id);
}

async fn bridge_handle_search(
    stream: &mut TcpStream,
    payload: Vec<u8>,
    state: &AppState,
) -> Result<(), &'static str> {
    let mut cursor = 0;
    let Some(query) = bridge_read_string(&payload, &mut cursor) else {
        return Err("invalid bridge search request");
    };
    let Some(token) = bridge_read_i32(&payload, &mut cursor) else {
        return Err("invalid bridge search token");
    };
    let query = query.to_ascii_lowercase();
    let username = state
        .config
        .username
        .clone()
        .unwrap_or_else(|| "slskR".to_owned());
    let entries = state.shares.read().await.entries.clone();
    let mut files = Vec::new();
    for entry in entries {
        if !entry.filename.to_ascii_lowercase().contains(&query) {
            continue;
        }
        files.push((entry.filename, entry.size, entry.extension));
        if files.len() >= 1_000 {
            break;
        }
    }
    let mut response = Vec::new();
    bridge_write_i32(&mut response, token);
    bridge_write_i32(
        &mut response,
        i32::try_from(files.len()).unwrap_or(i32::MAX),
    );
    for (filename, size, extension) in files {
        bridge_write_string(&mut response, &username);
        bridge_write_string(&mut response, &filename);
        response.extend_from_slice(&i64::try_from(size).unwrap_or(i64::MAX).to_le_bytes());
        bridge_write_i32(&mut response, 0);
        bridge_write_string(&mut response, &extension);
    }
    bridge_write_frame(stream, BRIDGE_SEARCH_RESPONSE, &response).await
}

async fn bridge_handle_download(
    stream: &mut TcpStream,
    payload: Vec<u8>,
    state: &AppState,
    session: &mut BridgeClientSession,
) -> Result<(), &'static str> {
    let mut cursor = 0;
    let Some(username) = bridge_read_string(&payload, &mut cursor) else {
        return Err("invalid bridge download username");
    };
    let Some(filename) = bridge_read_string(&payload, &mut cursor) else {
        return Err("invalid bridge download filename");
    };
    let Some(token) = bridge_read_i32(&payload, &mut cursor) else {
        return Err("invalid bridge download token");
    };
    let max_transfers = state
        .config
        .media_services
        .virtual_soulfind
        .bridge
        .max_transfers_per_session
        .max(1);
    if session.active_transfer_id.is_some() {
        bridge_write_frame(
            stream,
            BRIDGE_DOWNLOAD_RESPONSE,
            &bridge_download_wire_response(false, "A download is already active", token),
        )
        .await?;
        return Ok(());
    }
    if session.transfer_count >= max_transfers {
        bridge_write_frame(
            stream,
            BRIDGE_DOWNLOAD_RESPONSE,
            &bridge_download_wire_response(false, "Transfer quota exceeded", token),
        )
        .await?;
        return Ok(());
    }
    let request = serde_json::json!({
        "username": username,
        "filename": filename,
    })
    .to_string();
    match extended_controller_download_response(&request, state).await {
        Ok(requests) => {
            let transfer_id = requests
                .first()
                .map(|entry| {
                    entry
                        .request_id
                        .clone()
                        .unwrap_or_else(|| entry.id.to_string())
                })
                .unwrap_or_default();
            session.transfer_count = session.transfer_count.saturating_add(1);
            session.active_transfer_id = Some(transfer_id.clone());
            bridge_write_frame(
                stream,
                BRIDGE_DOWNLOAD_RESPONSE,
                &bridge_download_wire_response(true, &transfer_id, token),
            )
            .await
        }
        Err(response) => {
            bridge_write_frame(
                stream,
                BRIDGE_DOWNLOAD_RESPONSE,
                &bridge_download_wire_response(false, &response.body, token),
            )
            .await
        }
    }
}

async fn bridge_handle_room_list(
    stream: &mut TcpStream,
    state: &AppState,
) -> Result<(), &'static str> {
    let rooms = state
        .rooms
        .read()
        .await
        .records
        .iter()
        .filter(|room| room.joined)
        .map(|room| {
            (
                room.name.clone(),
                room.user_count
                    .unwrap_or_else(|| u32::try_from(room.members.len()).unwrap_or(u32::MAX)),
            )
        })
        .collect::<Vec<_>>();
    let mut response = Vec::new();
    bridge_write_i32(
        &mut response,
        i32::try_from(rooms.len()).unwrap_or(i32::MAX),
    );
    for (name, user_count) in rooms {
        bridge_write_string(&mut response, &name);
        bridge_write_i32(&mut response, i32::try_from(user_count).unwrap_or(i32::MAX));
    }
    bridge_write_frame(stream, BRIDGE_ROOM_LIST_RESPONSE, &response).await
}
