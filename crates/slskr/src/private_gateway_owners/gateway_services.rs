use super::*;

impl Gateway {
    pub(super) async fn handle_connection(
        &self,
        tcp: TcpStream,
        state: &crate::AppState,
    ) -> Result<(), String> {
        let remote_address = tcp
            .peer_addr()
            .map_err(|error| format!("overlay peer address failed: {error}"))?;
        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
            .await
            .map_err(|_| "overlay TLS accept timed out".to_owned())?
            .map_err(|error| format!("overlay TLS accept failed: {error}"))?;
        let certificate_thumbprint = tls
            .get_ref()
            .1
            .peer_certificates()
            .and_then(|certificates| certificates.first())
            .map(|certificate| hex::encode(Sha256::digest(certificate.as_ref())));
        let mut framer = OverlayFramer::new(tls);
        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
            .await
            .map_err(|_| "overlay hello timed out".to_owned())?
            .map_err(|error| format!("overlay hello failed: {error}"))?;
        hello
            .validate()
            .map_err(|error| format!("overlay hello rejected: {error}"))?;
        let supports_mesh_service = hello
            .features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SERVICE));
        let supports_mesh_search = hello
            .features
            .iter()
            .any(|feature| feature.eq_ignore_ascii_case(FEATURE_MESH_SEARCH));
        if !supports_mesh_service && !supports_mesh_search {
            return Err("overlay peer advertises no supported feature".to_owned());
        }
        authenticate_overlay_peer(state, &hello, remote_address.ip(), &self.certificate_sha256)
            .await?;
        let connection_id = uuid::Uuid::new_v4().simple().to_string();
        let local_username = crate::pod_request_peer_id(state)
            .await
            .ok_or_else(|| "local gateway identity is unavailable".to_owned())?;
        let features = [
            (FEATURE_MESH_SERVICE, supports_mesh_service),
            (FEATURE_MESH_SEARCH, supports_mesh_search),
        ]
        .into_iter()
        .filter_map(|(feature, supported)| supported.then_some(feature.to_owned()))
        .collect();
        let acknowledgement = MeshHelloAck {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_hello_ack".to_owned(),
            version: OVERLAY_VERSION,
            username: local_username,
            features,
            soulseek_ports: None,
            overlay_port: Some(self.bind().port()),
            nonce_echo: hello.nonce,
        };
        self.overlay_connections.write().await.insert(
            connection_id.clone(),
            OverlayConnectionMetadata {
                username: hello.username.clone(),
                address: remote_address.ip().to_string(),
                port: remote_address.port(),
                features: hello.features.clone(),
                connected_at: overlay_timestamp(),
                last_activity: overlay_timestamp(),
                certificate_thumbprint,
                version: hello.version,
                is_outbound: false,
            },
        );

        let result = async {
            framer
                .write(&acknowledgement)
                .await
                .map_err(|error| format!("overlay acknowledgement failed: {error}"))?;
            let mut liveness = OverlayLiveness::new();
            loop {
                if liveness.is_idle() {
                    return Err("overlay connection was idle too long".to_owned());
                }
                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
                    Ok(result) => {
                        liveness.record_inbound();
                        self.touch_overlay_connection(&connection_id).await;
                        result.map_err(|error| format!("overlay read failed: {error}"))?
                    }
                    Err(_) if liveness.last_ping.elapsed() >= OVERLAY_KEEPALIVE_INTERVAL => {
                        let timestamp = i64::try_from(crate::unix_timestamp_millis())
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
                if !self
                    .overlay_rate_limiter
                    .check_message(&connection_id)
                    .allowed
                {
                    return Err("overlay message rate exceeded".to_owned());
                }
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
                    "mesh_search_req" if supports_mesh_search => {
                        let request: MeshSearchRequestMessage = serde_json::from_slice(&raw)
                            .map_err(|error| {
                                format!("overlay mesh search request is invalid: {error}")
                            })?;
                        // The frozen dispatcher drops invalid requests after recording a
                        // violation; it does not manufacture a response for malformed input.
                        if request.validate().is_err() {
                            continue;
                        }
                        if !self
                            .overlay_rate_limiter
                            .check_mesh_search_request(&hello.username)
                            .allowed
                        {
                            return Err("overlay mesh search rate exceeded".to_owned());
                        }
                        let response = self.handle_mesh_search(request, state).await;
                        framer.write(&response).await.map_err(|error| {
                            format!("overlay mesh search response failed: {error}")
                        })?;
                    }
                    "mesh_search_req" => {
                        return Err("overlay mesh search is not negotiated".to_owned());
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
        self.overlay_rate_limiter.remove_connection(&connection_id);
        self.remove_connection_tunnels(&connection_id).await;
        self.overlay_connections
            .write()
            .await
            .remove(&connection_id);
        result
    }

    pub(super) async fn handle_mesh_search(
        &self,
        request: MeshSearchRequestMessage,
        state: &crate::AppState,
    ) -> MeshSearchResponseMessage {
        let request_id = request.request_id.clone();
        let search = timeout(Duration::from_secs(5), async {
            let entries = state.shares.read().await.entries.clone();
            let mut matches = crate::search_shares(&entries, &request.search_text);
            matches.sort_by(|left, right| left.filename.cmp(&right.filename));

            let max_results = usize::try_from(request.max_results).unwrap_or(1);
            let truncated = matches.len() > max_results;
            matches.truncate(max_results);
            let files = matches
                .iter()
                .filter_map(mesh_search_file_dto)
                .collect::<Vec<_>>();
            MeshSearchResponseMessage::new(request.request_id, files, truncated, None)
        })
        .await;

        match search {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                tracing::debug!(%error, "mesh search response validation failed");
                mesh_search_error_response(request_id, "Search failed")
            }
            Err(_) => mesh_search_error_response(request_id, "Search failed"),
        }
    }

    pub(super) async fn handle_call(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &crate::AppState,
    ) -> MeshServiceReply {
        self.handle_call_with_mode(call, remote_username, connection_id, state, false)
            .await
    }

    pub(super) async fn handle_call_with_mode(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &crate::AppState,
        local_http: bool,
    ) -> MeshServiceReply {
        let service_enabled = {
            let media_services = state.media_services.read().await;
            if local_http {
                local_service_enabled(call.service_name.as_str(), &media_services.features)
            } else {
                overlay_service_enabled(
                    call.service_name.as_str(),
                    &media_services.features,
                    state.config.controller_profile,
                )
            }
        };
        let result = if !valid_service_call(&call) {
            Err((4, "Invalid service call".to_owned()))
        } else if !service_enabled {
            Err((2, format!("Service '{}' not found", call.service_name)))
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
                "dht" => {
                    self.dht_service
                        .handle_call(&call.method, &call.payload, remote_username)
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

    /// Dispatch an HTTP-gateway service call through the same real service
    /// handlers used by authenticated overlay peers.  This keeps the local
    /// gateway from inventing a compatibility record when it is configured as
    /// a provider for the HTTP gateway.
    pub async fn call_http_service(
        &self,
        call: MeshServiceCall,
        remote_username: &str,
        connection_id: &str,
        state: &crate::AppState,
    ) -> MeshServiceReply {
        self.handle_call_with_mode(call, remote_username, connection_id, state, true)
            .await
    }

    pub(super) async fn handle_shadow_index_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &crate::AppState,
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

    pub(super) async fn handle_mesh_content_call(
        &self,
        method: &str,
        payload: &[u8],
        state: &crate::AppState,
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
                        || crate::stable_content_hash(&entry.filename, entry.size).to_string()
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
        let mut file = crate::open_shared_local_file(state, &local_path)
            .await
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

    pub(super) async fn handle_pods_call(
        &self,
        method: &str,
        payload: &[u8],
        remote_username: &str,
        state: &crate::AppState,
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
                        crate::unix_timestamp_millis(),
                    )
                    .map_err(|error| (1, error))?;
                if let Some(binding) =
                    binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
                {
                    let room = binding.identifier;
                    if let Err(error) = crate::try_send_session_command(
                        state,
                        crate::SessionCommand::SayRoom {
                            room: room.clone(),
                            body: format!("[Pod:{}] {}", message.sender_peer_id, message.body),
                        },
                    ) {
                        crate::record_pod_room_mirror_failure(state, &room, &error).await;
                    }
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

    pub(super) async fn open_tunnel(
        &self,
        payload: &[u8],
        remote_username: &str,
        connection_id: &str,
        state: &crate::AppState,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: OpenTunnelRequest = parse_payload(payload)?;
        let now = crate::unix_timestamp();
        if !valid_open_tunnel_request(&request)
            || request.request_timestamp < 0
            || now.abs_diff(request.request_timestamp as u64) > REQUEST_FRESHNESS_SECONDS
        {
            return Err((4, "Invalid tunnel request".to_owned()));
        }
        let local_username = crate::pod_request_peer_id(state)
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
        let peer_identity = gateway_peer_identity(remote_username);
        {
            let mut nonces = self.replay_nonces.lock().await;
            nonces.retain(|_, seen| now.saturating_sub(*seen) <= REQUEST_FRESHNESS_SECONDS);
            let key = gateway_replay_nonce_key(&peer_identity, &request.request_nonce);
            if nonces.contains_key(&key) {
                return Err((8, "Tunnel request nonce was replayed".to_owned()));
            }
            if nonces
                .keys()
                .filter(|(username, _)| username == &peer_identity)
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
                .filter(|tunnel| tunnel.owner == peer_identity)
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
                .filter(|tunnel| tunnel.owner == peer_identity)
                .count()
                >= MAX_TUNNELS_PER_PEER
        {
            return Err((6, "Tunnel capacity is full".to_owned()));
        }
        let reader_task = tokio::spawn(async move {
            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
            while let Ok(read) = reader.read(&mut buffer).await {
                if read == 0 || incoming_tx.send(buffer[..read].to_vec()).await.is_err() {
                    break;
                }
            }
        });
        let reader_abort = reader_task.abort_handle();
        drop(reader_task);
        tunnels.insert(
            tunnel_id.clone(),
            Arc::new(Tunnel {
                owner: peer_identity,
                connection_id: connection_id.to_owned(),
                pod_id: request.pod_id,
                writer: Mutex::new(writer),
                incoming: Mutex::new(incoming_rx),
                reader_abort,
            }),
        );
        drop(tunnels);
        serde_json::to_vec(&OpenTunnelResponse {
            tunnel_id,
            accepted: true,
        })
        .map_err(|_| (1, "Tunnel response failed".to_owned()))
    }

    pub(super) async fn tunnel_data(
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

    pub(super) async fn get_tunnel_data(
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

    pub(super) async fn close_tunnel(
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

    pub(super) async fn owned_tunnel(
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
        if tunnel.owner != gateway_peer_identity(remote_username)
            || tunnel.connection_id != connection_id
        {
            return Err((8, "Tunnel belongs to another peer".to_owned()));
        }
        Ok(tunnel)
    }

    pub(super) async fn remove_connection_tunnels(&self, connection_id: &str) {
        self.tunnels
            .write()
            .await
            .retain(|_, tunnel| tunnel.connection_id != connection_id);
    }

    pub(super) async fn touch_overlay_connection(&self, connection_id: &str) {
        if let Some(connection) = self
            .overlay_connections
            .write()
            .await
            .get_mut(connection_id)
        {
            connection.last_activity = overlay_timestamp();
        }
    }
}
