# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.
# Generated: 2026-10-04T16:53:08Z
# Source digest: 96eede3274f36c170aeabfde08506578f1ed9bb44ca6c941a4b6a77616118143

## Protocol-controlled allocations and lengths
crates/slskr-web/src/rustymilk_ui_owners/visualizer_audio_analysis.rs:21:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/rustymilk_ui_owners/visualizer_audio_analysis.rs:22:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr-protocol/src/peer.rs:727:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-protocol/src/peer.rs:740:        let compressed = compress_zlib(&vec![b'x'; MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES + 1])
crates/slskr-protocol/src/distributed.rs:114:                    payload: reader.read_bytes(reader.remaining())?.to_vec(),
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr/src/route_dispatch.rs:132:    let mut normalized = Vec::with_capacity(terms.len());
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr/src/transfer_state_io.rs:104:    let mut actual = vec![0_u8; HEADER.len()];
crates/slskr-protocol/src/primitives.rs:119:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:151:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:152:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:160:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:177:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:210:            output: Vec::with_capacity(capacity),
crates/slskr/src/soulfind_bridge_runtime.rs:55:    let mut payload = vec![0_u8; length - 4];
crates/slskr/src/soulfind_bridge_runtime.rs:163:    let mut provided_padded = vec![0_u8; length];
crates/slskr/src/soulfind_bridge_runtime.rs:164:    let mut configured_padded = vec![0_u8; length];
crates/slskr/src/mesh_services.rs:186:    let mut completed = Vec::with_capacity(manifest.len());
crates/slskr/src/mesh_services.rs:329:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/preview_stream_controller.rs:1008:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr-protocol/src/server.rs:919:                let payload = reader.read_bytes(reader.remaining())?.to_vec();
crates/slskr-protocol/src/server.rs:1824:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:1866:    let mut users = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:1977:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2013:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2069:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2118:    let mut entries = Vec::with_capacity(names.len());
crates/slskr/src/peer_transport.rs:15:    let mut order = Vec::with_capacity(2);
crates/slskr/src/peer_transport.rs:209:            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
crates/slskr/src/peer_transport.rs:288:    let mut bound_address_and_port = vec![0_u8; address_len + 2];
crates/slskr/src/realm_subject_index.rs:204:        let mut validated_keys = Vec::with_capacity(indexes.len());
crates/slskr/src/realm_subject_index.rs:1299:            serde_json::Value::Array(vec![entry; MAX_ENTRIES_PER_INDEX + 1]);
crates/slskr/src/realm_subject_index.rs:1306:            serde_json::json!(vec!["alias"; MAX_ALIASES_PER_ENTRY + 1]);
crates/slskr/src/songid_runtime.rs:376:        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
crates/slskr/src/songid_runtime.rs:378:            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
crates/slskr/src/songid_runtime.rs:380:        "url" => vec!["URL detected; using source query fallback.".to_owned()],
crates/slskr/src/quic_alpn.rs:177:    let mut output = vec![0_u8; length];
crates/slskr/src/quic_alpn.rs:190:    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
crates/slskr/src/mesh_dht.rs:1102:                &vec![b'x'; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr/src/mesh_dht.rs:1132:        let mut output = vec![0; MAX_DHT_VALUE_BYTES - 1];
crates/slskr/src/hash_backfill_runtime.rs:226:        connection.read_chunk(wanted),
crates/slskr/src/search_fallback.rs:37:    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
crates/slskr/src/bloom_filter.rs:72:            bits: vec![0_u8; bit_size.div_ceil(8)],
crates/slskr/src/webhooks.rs:1191:    let mut events = Vec::with_capacity(values.len());
crates/slskr/src/utils.rs:759:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:777:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:1109:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr/src/events_ws.rs:334:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:453:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:730:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/events_ws.rs:941:        let payload = vec![b'x'; 1024 * 1024];
crates/slskr/src/port_forwarding.rs:354:                let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/mediacore_mutations.rs:646:        let mut results = Vec::with_capacity(descriptors.len());
crates/slskr/src/mediacore_mutations.rs:800:        let mut results = Vec::with_capacity(ids.len());
crates/slskr/src/content_discovery.rs:238:        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
crates/slskr/src/content_discovery.rs:247:        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
crates/slskr/src/content_discovery.rs:361:        let mut normalized = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:660:        let mut valid = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:675:        let mut candidates = Vec::with_capacity(valid.len());
crates/slskr/src/content_discovery.rs:851:    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
crates/slskr/src/content_discovery.rs:943:    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:972:    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
crates/slskr/src/multisource.rs:531:        let mut sources = Vec::with_capacity(request.sources.len());
crates/slskr/src/multisource.rs:573:        let mut source_busy = vec![false; sources.len()];
crates/slskr/src/multisource.rs:577:        let mut results = Vec::with_capacity(chunks.len());
crates/slskr/src/multisource.rs:811:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/wishlist_store.rs:743:        let mut updated = Vec::with_capacity(distinct_ids.len());
crates/slskr/src/dotnet_regex.rs:340:    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
crates/slskr/src/dotnet_regex.rs:355:    let mut named_slots = Vec::with_capacity(named.len());
crates/slskr/src/storage.rs:331:    let mut entries = Vec::with_capacity(file_count);
crates/slskr/src/storage.rs:348:        let mut attributes = Vec::with_capacity(attr_count);
crates/slskr/src/extended_controller.rs:193:    let mut requested_files = Vec::with_capacity(files.len());
crates/slskr/src/cli.rs:1128:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1355:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/security_controls.rs:1923:        let mut transformed = Vec::with_capacity(transformed_len);
crates/slskr/src/extended_controller_get.rs:229:                let mut peers = Vec::with_capacity(peer_records.len());
crates/slskr/src/extended_controller_get.rs:755:                let mut entries = Vec::with_capacity(requests.len());
crates/slskr/src/relay_ws.rs:436:    let mut header = Vec::with_capacity(10);
crates/slskr/src/relay_ws.rs:519:    let mut payload = vec![0_u8; length as usize];
crates/slskr/src/relay_ws.rs:574:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/mediacore_controller.rs:212:        let mut current = Vec::with_capacity(right.len() + 1);
crates/slskr/src/share_backfill_controller.rs:74:            let mut manifest = Vec::with_capacity(items.len());
crates/slskr/src/share_backfill_controller.rs:143:                let mut bytes = vec![0_u8; length];
crates/slskr/src/mesh_sync.rs:119:            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
crates/slskr/src/mesh_sync.rs:238:    let mut incoming = Vec::with_capacity(received);
crates/slskr/src/mesh_sync.rs:352:async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
crates/slskr/src/mesh_sync.rs:432:    let mut data = vec![0_u8; to_read];
crates/slskr-client/benches/search_response_dedup.rs:125:    let mut current = Vec::with_capacity(TRIALS);
crates/slskr-client/benches/search_response_dedup.rs:126:    let mut linear = Vec::with_capacity(TRIALS);
crates/slskr-client/benches/file_transfer_payload.rs:135:            .read_chunk(length)
crates/slskr-client/benches/file_transfer_payload.rs:188:                let mut frame = Vec::with_capacity(4 + chunk.len());
crates/slskr-client/benches/file_transfer_payload.rs:243:    let payload = vec![PAYLOAD_BYTE; config.payload_bytes];
crates/slskr-client/benches/file_transfer_payload.rs:249:    let mut samples: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::with_capacity(config.trials));
crates/slskr/src/relay_agent.rs:775:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay_agent.rs:978:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/controller_capabilities.rs:569:            let mut bytes = Vec::with_capacity(33);
crates/slskr/src/relay.rs:429:        let mut shares = Vec::with_capacity(rows.len());
crates/slskr/src/relay.rs:1578:        let mut quotient = Vec::with_capacity(source.len());
crates/slskr/src/relay.rs:2060:        let records = vec![record.clone(); MAX_RELAY_SHARE_UPLOAD_RECORDS + 1];
crates/slskr/src/http_server.rs:456:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:560:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/http_server.rs:947:        let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/http_server.rs:1106:        let body = vec![b'x'; 100 * 1024];
crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs:137:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/share_scanner.rs:379:        let mut flac = vec![0; 42];
crates/slskr/src/web_static.rs:123:    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
crates/slskr-client/src/overlay/frame.rs:53:        let mut payload = vec![0_u8; length];
crates/slskr/src/activitypub_controller.rs:63:    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
crates/slskr/src/activitypub_controller.rs:176:    let mut lines = Vec::with_capacity(parsed.headers.len());
crates/slskr/src/podcore_mutations.rs:858:            let mut results = Vec::with_capacity(work.len());
crates/slskr/src/cli_smoke_soak_owners/peer_probe_operations.rs:196:        .read_chunk(5)
crates/slskr-client/src/overlay/client.rs:334:        let mut payload = vec![0; 15];
crates/slskr-client/src/overlay/client.rs:565:        let mut signature = vec![0_u8; 64];
crates/slskr-client/src/overlay/client.rs:762:                vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr-client/src/overlay/client.rs:774:            payload: vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr/src/lidarr_wishlist_sync.rs:41:        let mut records = Vec::with_capacity(raw_records.len());
crates/slskr/src/transfer_batch_controller.rs:447:    let mut staged = Vec::with_capacity(prepared.len());
crates/slskr/src/request_security.rs:182:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr-client/src/transfer.rs:208:            connection.read_chunk(remaining).await
crates/slskr-client/src/quic_data.rs:619:    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
crates/slskr-client/src/quic_data.rs:1018:        let received = receive.read_chunk(&mut buffer).await;
crates/slskr-client/src/quic_data.rs:1020:            .read_chunk(&mut buffer)
crates/slskr/src/library_controller.rs:249:    let mut file_values = Vec::with_capacity(page.len());
crates/slskr/src/library_controller.rs:423:    let mut items = Vec::with_capacity(candidates.len());
crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs:201:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr-client/src/mesh_sync.rs:434:        let mut output = Vec::with_capacity(encoded.len());
crates/slskr-client/src/mesh_sync.rs:1053:            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
crates/slskr-client/src/quic_control.rs:41:    let mut encoded = Vec::with_capacity(key_value_len + 5);
crates/slskr/src/file_transfer_runtime_owners/upload_streaming.rs:96:    let mut buffer = vec![0_u8; buffer_len];
crates/slskr-client/src/overlay_control.rs:77:        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
crates/slskr-client/src/overlay_control.rs:111:        let payload = reader.read_bytes("payload")?;
crates/slskr-client/src/overlay_control.rs:357:    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
crates/slskr-client/src/listener.rs:235:    let mut buffered = Vec::with_capacity(8);
crates/slskr-client/src/listener.rs:376:    let mut candidates = Vec::with_capacity(2);
crates/slskr-client/src/listener.rs:536:            let mut nested = Vec::with_capacity(nested_len);
crates/slskr-client/src/search.rs:630:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr/src/route_dispatch_group_2_transfer_status.rs:513:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/route_dispatch_group_2_transfer_status.rs:676:            let mut prepared = Vec::with_capacity(files.len());
crates/slskr/src/file_transfer_runtime_owners/download_progress.rs:42:            connection.read_chunk(next_len),
crates/slskr/src/route_dispatch_group_2_downloads.rs:206:                let mut prepared = Vec::with_capacity(files.len());
crates/slskr-client/src/capabilities.rs:173:        let mut features = Vec::with_capacity(feature_count);
crates/slskr-client/src/capabilities.rs:596:    String::from_utf8(reader.read_bytes(length)?.to_vec())
crates/slskr-client/src/capabilities.rs:617:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:668:    let mut output = Vec::with_capacity(values.len());
crates/slskr/src/legacy_route_dispatch_group_02.rs:568:                let mut prepared = Vec::with_capacity(files.len());
crates/slskr/src/legacy_route_dispatch_group_02.rs:1238:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/legacy_route_dispatch_group_02.rs:1370:             let mut prepared = Vec::with_capacity(files.len());
crates/slskr/src/file_transfer_runtime_owners/audio_metadata.rs:172:    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
crates/slskr/src/legacy_route_dispatch_group_05.rs:956:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/route_dispatch_group_4_notes_interests_grants.rs:485:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/private_gateway_owners/shared_quic_runtime.rs:101:        let mut buffer = vec![0_u8; 128 * 1024];
crates/slskr-client/src/io.rs:316:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:384:    let mut prefix = Vec::with_capacity(5);
crates/slskr-client/src/io.rs:642:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:674:    let mut obfuscated = Vec::with_capacity(encoded_len);
crates/slskr-client/src/file_transfer.rs:133:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:153:        let mut chunk = vec![0; length];
crates/slskr-client/src/file_transfer.rs:173:        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
crates/slskr-client/src/file_transfer.rs:198:        let mut payload = Vec::with_capacity(length);
crates/slskr-client/src/file_transfer.rs:222:        let mut encoded = Vec::with_capacity(first_block.len() + length);
crates/slskr/src/persistence_transfers.rs:168:        let mut records = Vec::with_capacity(ids.len());
crates/slskr/src/private_gateway_owners/quic_relay.rs:6:    let mut bytes = Vec::with_capacity(256);
crates/slskr/src/private_gateway_owners/quic_relay.rs:9:        let read = receive.read_chunk(&mut byte).await?;
crates/slskr/src/private_gateway_owners/quic_relay.rs:85:            .read_chunk(&mut buffer[..remaining])
crates/slskr/src/private_gateway_owners/quic_proxy.rs:31:                let mut response = vec![0_u8; 65_536];
crates/slskr/src/private_gateway_owners/tests.rs:209:    call.payload = vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1];
crates/slskr/src/private_gateway_owners/tests.rs:277:    let mut packet = vec![0_u8; 1_200];
crates/slskr/src/private_gateway_owners/tests.rs:529:        vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
crates/slskr/src/private_gateway_owners/gateway_services.rs:439:            let mut bytes = vec![0_u8; length];
crates/slskr/src/private_gateway_owners/gateway_services.rs:679:                let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/config_parts/peer_transport.rs:180:    let mut peers = Vec::with_capacity(values.len());

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/multisource.rs:707:    let mut builder = Client::builder()
crates/slskr/src/multisource.rs:708:        .redirect(Policy::none())
crates/slskr/src/multisource.rs:712:        builder = builder.resolve(host, SocketAddr::new(address.ip(), port));
crates/slskr/src/port_forwarding.rs:94:                "Port {} is already being forwarded",
crates/slskr/src/port_forwarding.rs:111:            bytes_forwarded: Arc::new(AtomicU64::new(0)),
crates/slskr/src/port_forwarding.rs:208:    bytes_forwarded: Arc<AtomicU64>,
crates/slskr/src/port_forwarding.rs:349:        let send_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:372:            let receive_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:423:        let bytes_forwarded = self.bytes_forwarded.load(Ordering::Relaxed);
crates/slskr/src/port_forwarding.rs:434:            bytes_forwarded,
crates/slskr/src/port_forwarding.rs:439:            performance: Performance::new(active_connections, bytes_forwarded),
crates/slskr/src/port_forwarding.rs:612:    pub bytes_forwarded: u64,
crates/slskr/src/webhooks.rs:593:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:594:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:877:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:878:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:881:            client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:125:    let forwarded = tree
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:129:    if forwarded != 1 {
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:131:            "distributed search reached {forwarded} children instead of one"
crates/slskr/src/application_state.rs:43:        "forwardedPort": runtime.vpn.forwarded_port,
crates/slskr/src/http_server.rs:67:    pub forwarded: Option<String>,
crates/slskr/src/http_server.rs:68:    pub x_forwarded_for: Option<String>,
crates/slskr/src/http_server.rs:123:                    "forwarded" => headers.forwarded = Some(value.to_string()),
crates/slskr/src/http_server.rs:124:                    "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
crates/slskr/src/http_server.rs:381:            "forwarded" => append_list_header(&mut headers.forwarded, value),
crates/slskr/src/http_server.rs:382:            "x-forwarded-for" => append_list_header(&mut headers.x_forwarded_for, value),
crates/slskr/src/http_server.rs:1068:            headers.forwarded,
crates/slskr/src/http_server.rs:1072:            headers.x_forwarded_for,
crates/slskr/src/http_server.rs:1269:            request.headers.x_forwarded_for.as_deref(),
crates/slskr/src/http_server.rs:1273:            request.headers.forwarded.as_deref(),
crates/slskr/src/musicbrainz_lookup.rs:123:    let client = reqwest::Client::builder()
crates/slskr/src/musicbrainz_lookup.rs:125:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/vpn.rs:20:    pub forwarded_port: Option<u16>,
crates/slskr/src/vpn.rs:153:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:172:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:187:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:238:    let client = reqwest::Client::builder()
crates/slskr/src/vpn.rs:239:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/vpn.rs:278:    let mut forwarded_port = None;
crates/slskr/src/vpn.rs:282:        forwarded_port = primary
crates/slskr/src/vpn.rs:316:                if forwarded_port.is_none() {
crates/slskr/src/vpn.rs:317:                    forwarded_port = port_forwards
crates/slskr/src/vpn.rs:327:        is_ready: !options.port_forwarding || forwarded_port.is_some(),
crates/slskr/src/vpn.rs:335:        forwarded_port,
crates/slskr/src/vpn.rs:439:        assert_eq!(status.forwarded_port, Some(44_444));
crates/slskr/src/vpn.rs:475:        assert_eq!(status.forwarded_port, Some(55_555));
crates/slskr/src/vpn.rs:542:        assert_eq!(status.forwarded_port, Some(45_678));
crates/slskr/src/controller_release_check.rs:158:        let client = reqwest::Client::builder()
crates/slskr/src/controller_release_check.rs:159:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/private_gateway_owners/gateway_udp_runtime.rs:99:                            "overlay QUIC proxy closed before initial datagram was forwarded"
crates/slskr/src/integration_target.rs:43:                .to_socket_addrs()
crates/slskr/src/integration_target.rs:60:        .to_socket_addrs()
crates/slskr/src/integration_target.rs:99:        .to_socket_addrs()
crates/slskr/src/private_gateway_owners/tests.rs:310:    .expect("DHT response should be forwarded")
crates/slskr/src/vpn_runtime.rs:33:                    "primary" => status.forwarded_port,
crates/slskr/src/vpn_runtime.rs:88:/// VPN's forwarded port. The local listener remains bound to the configured
crates/slskr/src/vpn_runtime.rs:98:            .forwarded_port
crates/slskr/src/lidarr_api.rs:16:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lidarr_api.rs:18:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lidarr_api.rs:21:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lidarr_api.rs:58:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lidarr_api.rs:60:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lidarr_api.rs:63:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lidarr_api.rs:191:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lidarr_api.rs:193:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lidarr_api.rs:196:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lidarr_api.rs:231:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lidarr_api.rs:233:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lidarr_api.rs:236:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lidarr_api.rs:270:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lidarr_api.rs:272:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lidarr_api.rs:275:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/relay_agent.rs:262:) -> Result<reqwest::Client, String> {
crates/slskr/src/relay_agent.rs:263:    let mut builder = reqwest::Client::builder()
crates/slskr/src/relay_agent.rs:264:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/relay_agent.rs:740:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:822:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:942:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:1003:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:1039:    client: &reqwest::Client,
crates/slskr/src/source_feed_ingest.rs:91:    let response = reqwest::Client::builder()
crates/slskr/src/source_feed_ingest.rs:93:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/source_feed_ingest.rs:365:        .to_socket_addrs()
crates/slskr/src/source_feed_ingest.rs:386:    let client = reqwest::Client::builder()
crates/slskr/src/source_feed_ingest.rs:388:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/private_gateway_owners/gateway_transport.rs:75:    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
crates/slskr/src/songid_runtime.rs:76:        .to_socket_addrs()
crates/slskr/src/songid_runtime.rs:86:    let client = reqwest::Client::builder()
crates/slskr/src/songid_runtime.rs:88:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/songid_runtime.rs:261:    let client = reqwest::Client::builder()
crates/slskr/src/songid_runtime.rs:263:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/activitypub_controller.rs:203:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/activitypub_controller.rs:205:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/activitypub_controller.rs:208:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/daemon_runtime_setup.rs:172:        reqwest::Client::builder()
crates/slskr/src/daemon_runtime_setup.rs:175:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/request_security.rs:39:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/request_security.rs:44:fn forwarded_client_ip(
crates/slskr/src/request_security.rs:49:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/request_security.rs:50:        forwarded_header_client_ips(value)?
crates/slskr/src/request_security.rs:52:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/request_security.rs:53:        x_forwarded_for_client_ips(value)?
crates/slskr/src/request_security.rs:56:    forwarded_ips
crates/slskr/src/request_security.rs:68:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/request_security.rs:71:        .map(parse_forwarded_ip_token)
crates/slskr/src/request_security.rs:76:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/request_security.rs:79:        .map(parse_forwarded_element_ip)
crates/slskr/src/request_security.rs:84:pub(super) fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
crates/slskr/src/request_security.rs:85:    let mut forwarded_ip = None;
crates/slskr/src/request_security.rs:91:        if forwarded_ip.is_some() {
crates/slskr/src/request_security.rs:94:        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
crates/slskr/src/request_security.rs:96:    forwarded_ip
crates/slskr/src/request_security.rs:99:pub(super) fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/feature_mutation_controller.rs:343:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/feature_mutation_controller.rs:345:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/notification_runtime.rs:19:    let client = reqwest::Client::builder()
crates/slskr/src/notification_runtime.rs:21:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/notification_runtime.rs:51:    let response = reqwest::Client::builder()
crates/slskr/src/notification_runtime.rs:53:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/notification_runtime.rs:78:    let client = reqwest::Client::builder()
crates/slskr/src/notification_runtime.rs:80:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/event_runtime.rs:96:            reqwest::Client::builder()
crates/slskr/src/event_runtime.rs:99:                .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/spotify_integration.rs:158:    let response = reqwest::Client::builder()
crates/slskr/src/spotify_integration.rs:160:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/spotify_integration.rs:179:    let response = reqwest::Client::builder()
crates/slskr/src/spotify_integration.rs:181:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/spotify_integration.rs:417:    let response = reqwest::Client::builder()
crates/slskr/src/spotify_integration.rs:419:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/spotify_integration.rs:471:    let response = reqwest::Client::builder()
crates/slskr/src/spotify_integration.rs:473:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/controller_yaml_validation.rs:1438:                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
crates/slskr/src/route_dispatch_group_7_network_admin.rs:240:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/route_dispatch_group_7_network_admin.rs:444:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:768:    parse_forwarded_element_ip, parse_forwarded_ip_token, rate_limit_user_key,
crates/slskr/src/legacy_route_dispatch_group_10.rs:597:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/legacy_route_dispatch_group_10.rs:804:                Err(error) if error.contains("already being forwarded") => {

## Filesystem and persistent-state boundaries
crates/slskr-client/benches/search_response_dedup.rs:168:            fs::create_dir_all(parent).expect("create benchmark artifact directory");
crates/slskr-client/benches/file_transfer_payload.rs:289:            fs::create_dir_all(parent)?;
crates/slskr/src/credential_store.rs:129:    let mut options = fs::OpenOptions::new();
crates/slskr/src/credential_store.rs:340:    fs::create_dir_all(parent).map_err(|error| {
crates/slskr/src/credential_store.rs:368:            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
crates/slskr/src/credential_store.rs:384:        fs::set_permissions(temporary_path, fs::Permissions::from_mode(0o600))
crates/slskr/src/credential_store.rs:424:        let mut options = OpenOptions::new();
crates/slskr/src/credential_store.rs:448:        fs::rename(&temporary_path, path)
crates/slskr/src/credential_store.rs:455:        let _ = fs::remove_file(&temporary_path);
crates/slskr/src/credential_store.rs:474:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:475:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:484:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:493:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:494:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:495:        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
crates/slskr/src/credential_store.rs:515:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:521:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:522:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:531:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:537:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:538:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:543:            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
crates/slskr/src/credential_store.rs:556:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:565:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:566:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:576:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:585:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:586:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:587:        fs::set_permissions(&root, fs::Permissions::from_mode(0o755))
crates/slskr/src/credential_store.rs:596:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:605:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:606:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:607:        fs::set_permissions(&root, fs::Permissions::from_mode(0o777))
crates/slskr/src/credential_store.rs:613:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:622:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:623:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:627:        fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
crates/slskr/src/credential_store.rs:632:        let _ = fs::remove_dir_all(root);
crates/slskr/src/credential_store.rs:641:        let _ = fs::remove_dir_all(&root);
crates/slskr/src/credential_store.rs:642:        fs::create_dir_all(&root).expect("create fixture directory");
crates/slskr/src/credential_store.rs:646:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/credential_store.rs:659:        let _ = fs::remove_dir_all(root);
crates/slskr/src/multisource.rs:72:            let _ = fs::remove_file(&self.path);
crates/slskr/src/multisource.rs:94:        let _ = fs::remove_dir_all(&self.path);
crates/slskr/src/multisource.rs:525:    fs::create_dir_all(parent).map_err(|_| "output directory could not be created".to_owned())?;
crates/slskr/src/multisource.rs:654:        let _ = fs::remove_file(&assembly_path);
crates/slskr/src/multisource.rs:843:    fs::remove_file(assembly_path)
crates/slskr/src/multisource.rs:872:    let mut options = fs::OpenOptions::new();
crates/slskr/src/multisource.rs:1221:        fs::remove_dir_all(root).expect("remove permissions test root");
crates/slskr/src/multisource.rs:1293:        fs::remove_dir_all(root).expect("remove swarm test root");
crates/slskr/src/multisource.rs:1386:        fs::remove_dir_all(root).expect("remove swarm cancellation test root");
crates/slskr/src/multisource.rs:1415:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/multisource.rs:1476:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/database_maintenance.rs:502:            match fs::remove_dir(&path) {
crates/slskr/src/database_maintenance.rs:537:                match fs::remove_file(&path) {
crates/slskr/src/database_maintenance.rs:758:                    match fs::remove_file(&path) {
crates/slskr/src/hash_backfill_state.rs:112:        let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1401:        let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1416:        fs::rename(&temporary_path, &manifest_path)
crates/slskr/src/relay.rs:1421:        let _ = fs::remove_file(&temporary_path);
crates/slskr/src/relay.rs:1501:    let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1912:            tokio::fs::remove_file(path)
crates/slskr/src/relay.rs:1947:        std::fs::create_dir_all(&root).expect("create relay share symlink fixture");
crates/slskr/src/relay.rs:1971:        std::fs::remove_dir_all(root).expect("remove relay share symlink fixture");
crates/slskr/src/relay.rs:1979:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:2010:        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
crates/slskr/src/relay.rs:2020:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:2042:        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
crates/slskr/src/relay.rs:2052:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:2070:        std::fs::remove_dir_all(root).expect("remove oversized manifest fixture");
crates/slskr/src/relay.rs:2080:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:2097:        std::fs::remove_dir_all(root).expect("remove invalid agent manifest fixture");
crates/slskr/src/relay.rs:2242:        std::fs::create_dir_all(&incoming).expect("create concurrent manifest directory");
crates/slskr/src/relay.rs:2291:        std::fs::remove_dir_all(root).expect("remove concurrent manifest fixture");
crates/slskr/src/lidarr_import.rs:190:    let directory = fs::canonicalize(directory)
crates/slskr/src/lidarr_import.rs:198:        fs::remove_file(&path).map_err(|error| {
crates/slskr/src/storage.rs:16:    fs::create_dir_all(parent)?;
crates/slskr/src/storage.rs:41:        let mut file = fs::OpenOptions::new()
crates/slskr/src/storage.rs:52:            let _ = fs::remove_file(temp_path);
crates/slskr/src/storage.rs:70:    fs::rename(source, destination)
crates/slskr/src/storage.rs:78:    match fs::remove_file(destination) {
crates/slskr/src/storage.rs:83:    fs::rename(source, destination)
crates/slskr/src/storage.rs:194:    OpenOptions::new()
crates/slskr/src/virtual_soulfind_v2.rs:583:        std::fs::remove_file(path).expect("remove executable catalogue fixture");
crates/slskr/src/transfer_state_io.rs:63:    fs::create_dir_all(parent)
crates/slskr/src/transfer_state_io.rs:72:    let mut options = fs::OpenOptions::new();
crates/slskr/src/transfer_state_io.rs:128:        fs::remove_file(&rotated_path)
crates/slskr/src/transfer_state_io.rs:131:    fs::rename(path, &rotated_path)
crates/slskr/src/transfer_state_io.rs:161:    let mut options = fs::OpenOptions::new();
crates/slskr/src/transfer_state_io.rs:268:    let mut options = fs::OpenOptions::new();
crates/slskr/src/persistence.rs:21:    let file = OpenOptions::new()
crates/slskr/src/persistence.rs:34:    file.set_permissions(std::fs::Permissions::from_mode(0o600))
crates/slskr/src/share_scanner.rs:757:        match root.canonicalize() {
crates/slskr/src/share_scanner.rs:852:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/controller_feature_state.rs:429:        std::fs::remove_file(state_path).unwrap();
crates/slskr/src/controller_feature_state.rs:443:                std::fs::rename(&mutation_path, &mutation_backup)
crates/slskr/src/controller_feature_state.rs:456:        std::fs::remove_dir(&state_path).unwrap();
crates/slskr/src/controller_feature_state.rs:457:        std::fs::rename(&backup_path, &state_path).unwrap();
crates/slskr/src/controller_feature_state.rs:467:        std::fs::remove_file(state_path).unwrap();
crates/slskr/src/ftp.rs:704:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:728:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:735:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:758:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:766:            tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:798:            tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:811:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:824:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:836:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:843:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:864:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:903:        tokio::fs::remove_file(file).await.unwrap();
crates/slskr/src/mesh_sync.rs:568:        let _ = std::fs::remove_file(path);
crates/slskr/src/relay_agent.rs:764:    fs::create_dir_all(&relay_directory)
crates/slskr/src/relay_agent.rs:799:    let cleanup = fs::remove_file(&database_path).await;
crates/slskr/src/relay_agent.rs:1130:    fs::rename(&temporary, &destination)
crates/slskr/src/relay_agent.rs:1139:    let mut options = fs::OpenOptions::new();
crates/slskr/src/relay_agent.rs:1184:            match std::fs::remove_file(&self.path) {
crates/slskr/src/relay_agent.rs:1304:        fs::remove_file(path)
crates/slskr/src/relay_agent.rs:1339:        std::fs::remove_file(path).unwrap();
crates/slskr/src/integration_runtime_state.rs:202:        fs::rename(&temporary, &path)
crates/slskr/src/integration_runtime_state.rs:264:            let _ = fs::remove_dir_all(&self.0);
crates/slskr/src/library_store.rs:99:            types: canonicalize(
crates/slskr/src/library_store.rs:112:            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
crates/slskr/src/library_store.rs:113:            statuses: canonicalize(
crates/slskr/src/mesh_security.rs:1311:                fs::create_dir_all(&mesh_directory)
crates/slskr/src/mesh_security.rs:1466:        let mut options = fs::OpenOptions::new();
crates/slskr/src/mesh_security.rs:2151:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/http_server.rs:1777:        std::fs::remove_file(path).unwrap();
crates/slskr/src/http_server.rs:1819:        std::fs::remove_file(path).unwrap();
crates/slskr/src/web_static.rs:144:        .canonicalize()
crates/slskr/src/web_static.rs:173:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/web_static.rs:196:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/web_static.rs:302:    let mut options = fs::OpenOptions::new();
crates/slskr/src/web_static.rs:355:    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/web_static.rs:356:    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/pod_channels.rs:459:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pod_channels.rs:559:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:581:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:590:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:615:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:624:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:649:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:658:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:682:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:691:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:707:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:110:        let mut options = fs::OpenOptions::new();
crates/slskr/src/realm_subject_index.rs:1374:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1390:        fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1403:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1412:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_reload.rs:33:    let mut options = fs::OpenOptions::new();
crates/slskr/src/controller_reload.rs:531:    fs::create_dir_all(parent)
crates/slskr/src/mesh_services.rs:71:            let _ = std::fs::remove_file(&self.path);
crates/slskr/src/mesh_services.rs:218:        let mut options = tokio::fs::OpenOptions::new();
crates/slskr/src/mesh_services.rs:267:        tokio::fs::remove_file(&staging_path)
crates/slskr/src/mesh_services.rs:598:    let mut options = tokio::fs::OpenOptions::new();
crates/slskr/src/mesh_services.rs:816:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:845:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:858:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:901:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:911:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:936:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:959:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:1000:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/transfer_recovery_runtime.rs:531:    let _ = fs::remove_file(output_path);
crates/slskr/src/versioned_relay_controller.rs:358:                                    let _ = fs::remove_file(&database_path);
crates/slskr/src/versioned_relay_controller.rs:364:                            let _ = fs::remove_file(&database_path);
crates/slskr/src/versioned_relay_controller.rs:380:    fs::create_dir_all(&directory)
crates/slskr/src/versioned_relay_controller.rs:391:    if fs::remove_file(path).is_err() {
crates/slskr/src/versioned_relay_controller.rs:433:    let mut options = fs::OpenOptions::new();
crates/slskr/src/versioned_relay_controller.rs:450:        let _ = fs::remove_file(path);
crates/slskr/src/versioned_relay_controller.rs:728:        std::fs::remove_dir_all(directory).expect("remove relay staging fixture");
crates/slskr/src/preview_stream_controller.rs:451:        .canonicalize()
crates/slskr/src/preview_stream_controller.rs:454:        let Ok(canonical_root) = root.canonicalize() else {
crates/slskr/src/preview_stream_controller.rs:521:    let mut options = fs::OpenOptions::new();
crates/slskr/src/preview_stream_controller.rs:573:    fs::create_dir_all(&directory)
crates/slskr/src/preview_stream_controller.rs:578:        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
crates/slskr/src/preview_stream_controller.rs:596:    let file = fs::OpenOptions::new()
crates/slskr/src/preview_stream_controller.rs:604:        let _ = fs::remove_file(&output_path);
crates/slskr/src/preview_stream_controller.rs:954:            let _ = fs::remove_file(&path);
crates/slskr/src/preview_stream_controller.rs:961:            let _ = fs::remove_file(&path);
crates/slskr/src/controller_storage.rs:47:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/controller_storage.rs:55:            .canonicalize()
crates/slskr/src/controller_storage.rs:57:        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
crates/slskr/src/controller_storage.rs:77:            fs::remove_dir_all(&path)
crates/slskr/src/controller_storage.rs:83:            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/controller_storage.rs:621:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/controller_storage.rs:638:            .canonicalize()
crates/slskr/src/controller_storage.rs:645:                .canonicalize()
crates/slskr/src/controller_storage.rs:650:                .canonicalize()
crates/slskr/src/http_connection.rs:1052:                let _ = fs::remove_file(path);
crates/slskr/src/event_runtime.rs:53:        match tokio::fs::create_dir_all(&log_dir).await {
crates/slskr/src/event_runtime.rs:55:                match tokio::fs::OpenOptions::new()
crates/slskr/src/content_discovery.rs:1001:    let mut options = fs::OpenOptions::new();
crates/slskr/src/content_discovery.rs:1379:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1403:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/content_discovery.rs:1412:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1431:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/controller_capabilities.rs:553:            let mut options = fs::OpenOptions::new();
crates/slskr/src/controller_capabilities.rs:579:                file.set_permissions(fs::Permissions::from_mode(0o600))
crates/slskr/src/controller_capabilities.rs:587:            let mut options = fs::OpenOptions::new();
crates/slskr/src/controller_capabilities.rs:601:            fs::rename(&temporary, &path)
crates/slskr/src/scripts.rs:105:    tokio::fs::create_dir_all(script_directory)
crates/slskr/src/daemon_runtime_setup.rs:104:        fs::remove_file(path)
crates/slskr/src/daemon_runtime_setup.rs:108:        fs::create_dir_all(parent)
crates/slskr/src/daemon_runtime_setup.rs:113:    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
crates/slskr/src/daemon_runtime_setup.rs:213:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/daemon_runtime_setup.rs:225:        std::fs::create_dir_all(path)
crates/slskr/src/daemon_runtime_setup.rs:272:    std::fs::create_dir_all(path).map_err(|error| {
crates/slskr/src/pods.rs:1437:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pods.rs:1580:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1593:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1602:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1608:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1617:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1631:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1671:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1700:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1709:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1734:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1743:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1766:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1775:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1807:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1816:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1841:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1850:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1915:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1924:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1937:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1946:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1971:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1980:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1998:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/songid_runtime.rs:236:    let _ = fs::remove_file(&normalized_path);
crates/slskr/src/spotify_integration.rs:45:    let mut options = fs::OpenOptions::new();
crates/slskr/src/spotify_integration.rs:138:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/spotify_integration.rs:146:    match fs::remove_file(path) {
crates/slskr/src/lib.rs:1453:    match tokio::fs::remove_file(path).await {
crates/slskr/src/lib.rs:1731:        match existing.canonicalize() {
crates/slskr/src/lib.rs:1777:    let writable = fs::OpenOptions::new()
crates/slskr/src/lib.rs:1783:        let _ = fs::remove_file(probe);
crates/slskr/src/lib.rs:1806:            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
crates/slskr/src/lib.rs:1888:            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
crates/slskr/src/config_yaml.rs:832:    let mut options = fs::OpenOptions::new();
crates/slskr/src/route_dispatch_group_2_transfer_files.rs:564:    match tokio::fs::remove_file(path).await {
crates/slskr/src/local_stream_file.rs:36:        if let Err(error) = fs::remove_file(&self.0) {
crates/slskr/src/local_stream_file.rs:75:                fs::remove_file(&cleanup).unwrap();
crates/slskr/src/core_dump_process.rs:78:            let _ = std::fs::remove_file(&self.output_path);
crates/slskr/src/core_dump_process.rs:139:                std::fs::remove_file(path).unwrap();
crates/slskr/src/config_tests/web_security.rs:13:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:29:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/web_security.rs:75:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:104:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/web_security.rs:155:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:180:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/web_security.rs:227:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:255:    std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config_tests/web_security.rs:268:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/web_security.rs:312:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:331:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/web_security.rs:344:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/web_security.rs:371:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:17:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:34:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:45:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:64:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:154:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:209:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:284:    std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:304:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/transfer_policy.rs:402:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:436:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:449:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/transfer_policy.rs:476:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:170:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:171:    std::fs::create_dir_all(&content).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:268:    std::fs::remove_dir_all(&content).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:269:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:296:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/runtime_policy.rs:365:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:213:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:256:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:599:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:653:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:708:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:879:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:888:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/peer_transport.rs:903:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:117:    std::fs::create_dir_all(&yaml_downloads).unwrap();
crates/slskr/src/config_tests/file_layers.rs:118:    std::fs::create_dir_all(&yaml_incomplete).unwrap();
crates/slskr/src/config_tests/file_layers.rs:119:    std::fs::create_dir_all(&yaml_share_a).unwrap();
crates/slskr/src/config_tests/file_layers.rs:120:    std::fs::create_dir_all(&yaml_share_b).unwrap();
crates/slskr/src/config_tests/file_layers.rs:121:    std::fs::create_dir_all(&env_downloads).unwrap();
crates/slskr/src/config_tests/file_layers.rs:183:    std::fs::create_dir_all(&relative_root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:213:    std::fs::remove_dir_all(relative_root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:214:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:227:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:312:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/file_layers.rs:325:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:362:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/file_layers.rs:375:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:397:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/file_layers.rs:413:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:424:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/file_layers.rs:425:    let _ = std::fs::remove_file(outside);
crates/slskr/src/config_tests/file_layers.rs:447:    let _ = std::fs::remove_file(path);
crates/slskr/src/config_tests/file_layers.rs:466:    let _ = std::fs::remove_dir(path);
crates/slskr/src/config_tests/file_layers.rs:482:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/file_layers.rs:491:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/file_layers.rs:509:    let _ = std::fs::remove_file(path);
crates/slskr/src/config_tests/media_integrations.rs:13:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:50:    std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:69:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:118:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:159:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/media_integrations.rs:172:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:221:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config_tests/media_integrations.rs:255:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:387:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:396:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config_tests/media_integrations.rs:411:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/gateway_identity.rs:35:    fs::create_dir_all(state_dir)
crates/slskr/src/private_gateway_owners/gateway_identity.rs:61:        return match fs::remove_file(certificate_path) {
crates/slskr/src/private_gateway_owners/gateway_identity.rs:90:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway_owners/gateway_identity.rs:133:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway_owners/gateway_identity.rs:145:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway_owners/gateway_identity.rs:150:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway_owners/gateway_identity.rs:153:    if let Err(error) = fs::remove_file(&temporary) {
crates/slskr/src/config_parts/file_loading.rs:31:    let file = fs::OpenOptions::new()
crates/slskr/src/config_parts/file_loading.rs:37:    fs::remove_file(&probe).map_err(|_| format!("{field} writeability probe cleanup failed"))?;
crates/slskr/src/config_parts/file_loading.rs:115:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway_owners/tests.rs:10:    fs::create_dir_all(&path).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:410:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:436:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:470:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:512:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:521:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:535:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:550:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:561:    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:566:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:582:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:596:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:615:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway_owners/tests.rs:786:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/file_transfer_runtime_owners/download_content_safety.rs:104:        fs::create_dir_all(&root)
crates/slskr/src/file_transfer_runtime_owners/download_content_safety.rs:111:        fs::rename(path, destination)
crates/slskr/src/file_transfer_runtime_owners/download_content_safety.rs:114:        fs::remove_file(path)
crates/slskr/src/file_transfer_runtime_owners/download_connection.rs:91:            fs::OpenOptions::new()
crates/slskr/src/file_transfer_runtime_owners/download_connection.rs:156:        fs::rename(&final_path, &incomplete_path)
crates/slskr/src/file_transfer_runtime_owners/download_connection.rs:184:        fs::remove_file(&completed_path)
crates/slskr/src/file_transfer_runtime_owners/download_connection.rs:187:    match fs::rename(&incomplete_path, &completed_path) {
crates/slskr/src/file_transfer_runtime_owners/download_connection.rs:195:            fs::remove_file(&incomplete_path)
crates/slskr/src/file_transfer_runtime_owners/completed_permissions.rs:31:        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
crates/slskr/src/file_transfer_runtime_owners/completed_permissions.rs:43:            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:355:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:363:        fs::create_dir_all(parent)
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:367:        .canonicalize()
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:372:        .canonicalize()
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:464:    let mut options = fs::OpenOptions::new();
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:515:        .canonicalize()
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:518:        .canonicalize()
crates/slskr/src/file_transfer_runtime_owners/download_paths.rs:523:    fs::OpenOptions::new()

## Async task and channel lifecycle boundaries
crates/slskr-client/benches/file_transfer_payload.rs:156:    let receiver = tokio::spawn(async move {
crates/slskr-client/src/shared_quic_server.rs:105:        let connection = tokio::time::timeout(Duration::from_secs(10), incoming)
crates/slskr-client/src/overlay/client.rs:12:    let tcp = timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(endpoint))
crates/slskr-client/src/overlay/client.rs:29:    let tls = timeout(TLS_HANDSHAKE_TIMEOUT, connector.connect(server_name, tcp))
crates/slskr-client/src/overlay/client.rs:40:    let mut client = timeout(
crates/slskr-client/src/overlay/client.rs:152:        self.call_with_timeout(call, SERVICE_CALL_TIMEOUT).await
crates/slskr-client/src/overlay/client.rs:159:        self.search_with_timeout(request, SERVICE_CALL_TIMEOUT)
crates/slskr-client/src/overlay/client.rs:163:    pub async fn call_with_timeout(
crates/slskr-client/src/overlay/client.rs:179:        match timeout(deadline, self.call_inner(call)).await {
crates/slskr-client/src/overlay/client.rs:233:    pub async fn search_with_timeout(
crates/slskr-client/src/overlay/client.rs:249:        match timeout(deadline, self.search_inner(request)).await {
crates/slskr-client/src/overlay/client.rs:325:        let task = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:349:        let writer = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:353:        let decoded = timeout(
crates/slskr-client/src/overlay/client.rs:588:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:669:        assert!(timeout(Duration::from_millis(10), wire.read_u8())
crates/slskr-client/src/overlay/client.rs:874:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:940:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:1104:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:1151:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:1166:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay/client.rs:1196:            .call_with_timeout(&call, Duration::from_millis(10))
crates/slskr-client/src/overlay/client.rs:1205:                .call_with_timeout(&call, Duration::from_secs(1))
crates/slskr-client/src/transfer.rs:156:        self.receive_file_from_with_timeout(
crates/slskr-client/src/transfer.rs:204:        let result = time::timeout(timeout, async {
crates/slskr-client/src/transfer.rs:451:        self.send_file_to_with_timeout(connection, bytes, DEFAULT_TRANSFER_IO_TIMEOUT)
crates/slskr-client/src/transfer.rs:481:        let result = time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:80:        self.accept_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:84:    pub async fn accept_with_timeout(
crates/slskr-client/src/listener.rs:88:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:106:        self.accept_obfuscated_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:110:    pub async fn accept_obfuscated_with_timeout(
crates/slskr-client/src/listener.rs:114:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:131:        self.accept_shared_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:135:    pub async fn accept_shared_with_timeout(
crates/slskr-client/src/listener.rs:139:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:156:        self.accept_shared_mesh_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:160:    pub async fn accept_shared_mesh_with_timeout(
crates/slskr-client/src/listener.rs:164:        time::timeout(timeout, async {
crates/slskr-client/src/peer_cache.rs:125:        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:129:    pub async fn send_to_with_timeout(
crates/slskr-client/src/peer_cache.rs:146:        match time::timeout(timeout, active.send(message)).await {
crates/slskr-client/src/peer_cache.rs:167:        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:171:    pub async fn receive_from_with_timeout(
crates/slskr-client/src/peer_cache.rs:187:        match time::timeout(timeout, active.receive()).await {
crates/slskr-client/src/stream.rs:35:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:42:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/distributed_tree.rs:343:        self.send_branch_info_to_parent_with_timeout(DEFAULT_DISTRIBUTED_IO_TIMEOUT)
crates/slskr-client/src/distributed_tree.rs:347:    pub async fn send_branch_info_to_parent_with_timeout(
crates/slskr-client/src/distributed_tree.rs:359:        let result = time::timeout(timeout, async {
crates/slskr-client/src/distributed_tree.rs:385:        self.forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:393:    pub async fn forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:406:        let result = time::timeout(timeout, async {
crates/slskr-client/src/search.rs:82:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:129:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:130:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/search.rs:160:    pub fn set_server_interval(&mut self, seconds: Option<u64>) {
crates/slskr-client/src/manager.rs:128:        self.ensure_peer_messages_with_timeout(username, DEFAULT_MANAGER_CONNECT_TIMEOUT)
crates/slskr-client/src/manager.rs:132:    pub async fn ensure_peer_messages_with_timeout(
crates/slskr-client/src/manager.rs:142:        time::timeout(timeout, async {
crates/slskr-client/src/manager.rs:190:        self.request_indirect_with_timeout(username, kind, DEFAULT_MANAGER_REQUEST_TIMEOUT)
crates/slskr-client/src/manager.rs:194:    pub async fn request_indirect_with_timeout(
crates/slskr-client/src/manager.rs:206:        match time::timeout(timeout, async {
crates/slskr-client/src/quic_data.rs:147:        Some(match timeout(QUIC_CONNECT_TIMEOUT, incoming).await {
crates/slskr-client/src/quic_data.rs:726:    timeout(QUIC_CONNECT_TIMEOUT, endpoint_client.wait_idle())
crates/slskr-client/src/quic_data.rs:764:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_data.rs:904:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:951:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:997:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:1049:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:1090:        let server = tokio::spawn(async move {
crates/slskr-client/src/peer_connect.rs:210:    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:238:    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:266:    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:295:    time::timeout(timeout, future)
crates/slskr/src/rate_limit.rs:136:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/events_ws.rs:130:            let frame = read_client_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/events_ws.rs:363:    time::timeout(timeout, read_client_frame(reader))
crates/slskr/src/events_ws.rs:432:    write_frame_with_timeout(writer, opcode, payload, WEBSOCKET_WRITE_TIMEOUT).await
crates/slskr/src/events_ws.rs:444:    time::timeout(timeout, write_frame_inner(writer, opcode, payload))
crates/slskr/src/events_ws.rs:601:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:606:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:635:        let message = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/events_ws.rs:656:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:661:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:712:        let message = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/events_ws.rs:724:        assert!(time::timeout(Duration::from_millis(100), socket.next())
crates/slskr/src/events_ws.rs:870:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:893:        let (event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:917:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:920:        let error = time::timeout(
crates/slskr/src/events_ws.rs:943:            write_frame_with_timeout(&mut writer, 0x82, &payload, Duration::from_millis(50))
crates/slskr/src/events_ws.rs:952:        let error = time::timeout(
crates/slskr/src/events_ws.rs:954:            read_client_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr/src/multisource.rs:710:        .timeout(SOURCE_TIMEOUT);
crates/slskr/src/multisource.rs:750:    timeout(deadline, resolution)
crates/slskr/src/multisource.rs:984:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:990:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:1040:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1344:        timeout(Duration::from_secs(5), first_stalled)
crates/slskr/src/multisource.rs:1348:        timeout(Duration::from_secs(5), second_stalled)
crates/slskr/src/multisource.rs:1425:        let server = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1451:        let fetch = tokio::spawn(async move {
crates/slskr/src/batch.rs:410:    fn test_batch_rejects_invalid_timeout() {
crates/slskr/src/preview_stream_controller.rs:562:    tokio::task::spawn_blocking(move || {
crates/slskr/src/preview_stream_controller.rs:982:        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
crates/slskr/src/preview_stream_controller.rs:989:        time::timeout(io_timeout, preview.connection.send_offset(0))
crates/slskr/src/preview_stream_controller.rs:999:    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
crates/slskr/src/preview_stream_controller.rs:1008:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/preview_stream_controller.rs:1015:            time::timeout(io_timeout, writer.write_all(&chunk))
crates/slskr/src/preview_stream_controller.rs:1022:    time::timeout(io_timeout, writer.flush())
crates/slskr/src/preview_stream_controller.rs:1044:    time::timeout(io_timeout, async {
crates/slskr/src/preview_stream_controller.rs:1242:        let request = tokio::spawn(run_application_dump_worker(move || {
crates/slskr/src/preview_stream_controller.rs:1245:                .recv_timeout(Duration::from_secs(5))
crates/slskr/src/preview_stream_controller.rs:1249:        tokio::time::timeout(Duration::from_secs(5), started_rx)
crates/slskr/src/preview_stream_controller.rs:1260:        tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/controller_regex.rs:12:    pub(crate) fn compile_with_timeout(
crates/slskr/src/controller_regex.rs:30:                .is_match_with_timeout(value, timeout)
crates/slskr/src/controller_regex.rs:39:fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
crates/slskr/src/controller_regex.rs:48:    let match_timeout = controller_regex_timeout(target);
crates/slskr/src/controller_regex.rs:52:            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
crates/slskr/src/mesh_services.rs:148:    timeout(
crates/slskr/src/mesh_services.rs:717:    timeout(deadline, operation)
crates/slskr/src/mesh_services.rs:863:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:877:        let fetch = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:964:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:978:        let fetch = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:120:        let task = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:188:        if timeout(Duration::from_secs(5), &mut task.0).await.is_err() {
crates/slskr/src/port_forwarding.rs:262:        if timeout(Duration::from_secs(3), async {
crates/slskr/src/port_forwarding.rs:405:            match timeout(TUNNEL_CLOSE_TIMEOUT, close_tunnel(&client, &tunnel_id)).await {
crates/slskr/src/port_forwarding.rs:551:    let reply = timeout(SERVICE_CALL_TIMEOUT, async {
crates/slskr/src/relay_ws.rs:51:    let handshake = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/relay_ws.rs:112:            let frame = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/relay_ws.rs:120:    let mut keepalive = time::interval(SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_ws.rs:424:    time::timeout(
crates/slskr/src/relay_ws.rs:564:    time::timeout(timeout, read_ws_frame(reader))
crates/slskr/src/relay_ws.rs:585:        let error = time::timeout(
crates/slskr/src/relay_ws.rs:587:            read_ws_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr/src/dht.rs:214:        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
crates/slskr/src/dht.rs:227:                match timeout(
crates/slskr/src/dht.rs:272:        timeout(self.lookup_timeout, async {
crates/slskr/src/dht.rs:451:            tokio::time::timeout(Duration::from_secs(1), remote.recv_from(&mut buffer))
crates/slskr/src/dht.rs:485:            tokio::time::timeout(Duration::from_secs(1), forwarder.recv_from(&mut buffer))
crates/slskr/src/dht.rs:580:        let peers = timeout(Duration::from_secs(10), async {
crates/slskr/src/dotnet_regex.rs:83:    pub fn is_match_with_timeout(&self, value: &str, timeout: Duration) -> Result<bool, String> {
crates/slskr/src/dotnet_regex.rs:107:        match receiver.recv_timeout(timeout) {
crates/slskr/src/scripts.rs:23:fn format_timeout(duration: Duration) -> String {
crates/slskr/src/scripts.rs:95:    run_with_timeout(script, script_directory, target, payload, SCRIPT_TIMEOUT).await
crates/slskr/src/scripts.rs:98:async fn run_with_timeout(
crates/slskr/src/scripts.rs:130:    let output = time::timeout(timeout_duration, async {
crates/slskr/src/scripts.rs:147:            format_timeout(timeout_duration)
crates/slskr/src/webhooks.rs:619:                .timeout(timeout)
crates/slskr/src/webhooks.rs:891:            .timeout(request_timeout)
crates/slskr/src/webhooks.rs:1025:    tokio::time::timeout(timeout, resolution)
crates/slskr/src/musicbrainz_lookup.rs:124:        .timeout(timeout)
crates/slskr/src/ftp.rs:232:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:240:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:254:            let ftp = tokio::time::timeout(timeout, AsyncRustlsFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:258:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:276:            if let Ok(Ok(ftp)) = tokio::time::timeout(timeout, secure).await {
crates/slskr/src/ftp.rs:279:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:329:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:548:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:583:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:888:            tokio::time::timeout(Duration::from_millis(50), listener.accept())
crates/slskr/src/ftp.rs:893:        let attempted = tokio::spawn(async move {
crates/slskr/src/managed_tasks.rs:83:        let _ = time::timeout(crate::MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, tasks.shutdown()).await;
crates/slskr/src/managed_tasks.rs:133:        tokio::time::timeout(std::time::Duration::from_secs(1), async {
crates/slskr/src/controller_feature_state.rs:55:        tokio::task::spawn_blocking(move || {
crates/slskr/src/controller_feature_state.rs:380:        let first = tokio::spawn(async move {
crates/slskr/src/controller_feature_state.rs:391:        let read = tokio::time::timeout(Duration::from_secs(2), store.read())
crates/slskr/src/controller_feature_state.rs:402:        let second = tokio::spawn(async move {
crates/slskr/src/controller_feature_state.rs:411:        tokio::time::timeout(Duration::from_secs(5), second)
crates/slskr/src/mesh_sync.rs:387:        tokio::task::spawn_blocking(move || read_file_chunk(file, offset, length, indexed_size))
crates/slskr/src/controller_release_check.rs:160:            .timeout(Duration::from_secs(100))
crates/slskr/src/vpn.rs:241:        .timeout(Duration::from_millis(options.gluetun.timeout))
crates/slskr/src/vpn.rs:370:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:415:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:455:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:490:            let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:515:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:554:        let server = tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:81:    let relay_target = time::timeout(
crates/slskr/src/relay_agent.rs:102:    let mut socket = time::timeout(
crates/slskr/src/relay_agent.rs:116:    let challenge = time::timeout(RELAY_REQUEST_TIMEOUT, wait_for_challenge(&mut socket))
crates/slskr/src/relay_agent.rs:135:    time::timeout(
crates/slskr/src/relay_agent.rs:144:    let share_token = time::timeout(
crates/slskr/src/relay_agent.rs:180:            messages = time::timeout(
crates/slskr/src/relay_agent.rs:265:        .timeout(RELAY_REQUEST_TIMEOUT)
crates/slskr/src/relay_agent.rs:556:    time::timeout(
crates/slskr/src/signalr_ws.rs:130:        relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/signalr_ws.rs:161:                relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT)
crates/slskr/src/signalr_ws.rs:170:    let mut keepalive = tokio::time::interval(relay_ws::SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/persistence.rs:1126:            .busy_timeout(Duration::from_secs(30));
crates/slskr/src/http_server.rs:189:    read_http_request_with_timeout(reader, REQUEST_READ_TIMEOUT, body_size_limit).await
crates/slskr/src/http_server.rs:197:    time::timeout(timeout, read_http_request_inner(reader, body_size_limit))
crates/slskr/src/http_server.rs:457:        time::timeout(BODY_READ_TIMEOUT, reader.read_exact(&mut buf))
crates/slskr/src/http_server.rs:669:        let available = time::timeout(timeout, reader.fill_buf())
crates/slskr/src/http_server.rs:709:    write_http_response_with_timeout(
crates/slskr/src/http_server.rs:729:    time::timeout(
crates/slskr/src/http_server.rs:744:    time::timeout(
crates/slskr/src/http_server.rs:895:                time::timeout(RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/http_server.rs:930:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(headers.as_bytes()))
crates/slskr/src/http_server.rs:938:            time::timeout(
crates/slskr/src/http_server.rs:951:            let read = time::timeout(RESPONSE_WRITE_TIMEOUT, file.read(&mut buffer[..wanted]))
crates/slskr/src/http_server.rs:958:            time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(&buffer[..read]))
crates/slskr/src/http_server.rs:965:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.flush())
crates/slskr/src/http_server.rs:1604:        tokio::spawn(async move {
crates/slskr/src/http_server.rs:1612:        let error = read_http_request_with_timeout(
crates/slskr/src/http_server.rs:1687:        let error = write_http_response_with_timeout(
crates/slskr/src/http_server.rs:1855:        tokio::spawn(async move {
crates/slskr/src/lidarr_api.rs:17:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lidarr_api.rs:59:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lidarr_api.rs:192:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lidarr_api.rs:232:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lidarr_api.rs:271:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/transfer_state_io.rs:45:            .recv_timeout(std::time::Duration::from_secs(5))
crates/slskr/src/share_index_runtime.rs:131:    let snapshot = tokio::task::spawn_blocking(move || {
crates/slskr/src/source_discovery_runtime.rs:66:        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
crates/slskr/src/database_maintenance.rs:680:    let prune_result = match tokio::task::spawn_blocking(move || {
crates/slskr/src/database_maintenance.rs:790:        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
crates/slskr/src/source_feed_ingest.rs:92:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/source_feed_ingest.rs:387:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/peer_message_runtime.rs:24:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/peer_message_runtime.rs:623:    let response = time::timeout(
crates/slskr/src/peer_message_runtime.rs:690:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/peer_message_runtime.rs:729:    time::timeout(
crates/slskr/src/peer_message_runtime.rs:742:    time::timeout(
crates/slskr/src/web_static.rs:452:    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/app_state.rs:401:        match time::timeout(MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, snapshot.save(db)).await {
crates/slskr/src/notification_runtime.rs:20:        .timeout(Duration::from_secs(30))
crates/slskr/src/notification_runtime.rs:52:        .timeout(Duration::from_secs(30))
crates/slskr/src/notification_runtime.rs:79:        .timeout(Duration::from_secs(30))
crates/slskr/src/distributed_runtime.rs:109:    let stream = time::timeout(
crates/slskr/src/distributed_runtime.rs:261:            received = time::timeout(
crates/slskr/src/distributed_runtime.rs:288:                    if time::timeout(
crates/slskr-client/src/quic_control.rs:253:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:333:        Some(match timeout(QUIC_CONNECT_TIMEOUT, incoming).await {
crates/slskr-client/src/quic_control.rs:410:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:428:    timeout(QUIC_CONNECT_TIMEOUT, endpoint_client.wait_idle())
crates/slskr-client/src/quic_control.rs:475:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:522:        let server = tokio::spawn(async move {
crates/slskr/src/listener_runtime.rs:384:    let incoming = match time::timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT, async move {
crates/slskr/src/listener_runtime.rs:440:    let incoming = match time::timeout(
crates/slskr/src/daemon_runtime_setup.rs:170:    let response = time::timeout(
crates/slskr/src/route_dispatch_group_2_search_rooms.rs:232:            let interests = match time::timeout(
crates/slskr/src/transfer_recovery_runtime.rs:149:        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
crates/slskr/src/spotify_integration.rs:159:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/spotify_integration.rs:180:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/spotify_integration.rs:418:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/spotify_integration.rs:472:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/versioned_relay_controller.rs:544:    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
crates/slskr/src/versioned_relay_controller.rs:606:    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
crates/slskr/src/cli.rs:565:    let stream = time::timeout(
crates/slskr/src/cli.rs:601:            let stream = time::timeout(
crates/slskr/src/cli.rs:659:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:665:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:735:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:756:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:770:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:785:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:792:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:843:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:857:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:884:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:895:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:944:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1066:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1116:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1128:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1184:            let _ = time::timeout(Duration::from_millis(750), peer.receive()).await;
crates/slskr/src/cli.rs:1196:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1339:        let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1355:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1599:            let stream = time::timeout(
crates/slskr/src/cli.rs:1785:        match time::timeout(remaining, session.receive()).await {
crates/slskr/src/cli.rs:1853:        match time::timeout(remaining, session.receive()).await {
crates/slskr/src/cli.rs:1885:    match time::timeout(Duration::from_secs(2), session.receive()).await {
crates/slskr/src/cli.rs:1932:            let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:1997:        match time::timeout(remaining, distributed.receive()).await {
crates/slskr/src/cli.rs:2038:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:2052:    let echoed = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli.rs:2236:            accept_result = listener.accept_with_timeout(remaining.min(Duration::from_secs(3))) => {
crates/slskr/src/legacy_route_dispatch_group_01.rs:871:                 tokio::spawn(async move {
crates/slskr/src/daemon_runtime_shutdown.rs:7:    let elapsed = shutdown_with_timeout(runtime, BLOCKING_WORK_SHUTDOWN_TIMEOUT);
crates/slskr/src/daemon_runtime_shutdown.rs:15:fn shutdown_with_timeout(runtime: Runtime, timeout: Duration) -> Duration {
crates/slskr/src/daemon_runtime_shutdown.rs:17:    runtime.shutdown_timeout(timeout);
crates/slskr/src/daemon_runtime_shutdown.rs:36:        runtime.spawn_blocking(move || {
crates/slskr/src/daemon_runtime_shutdown.rs:42:            .recv_timeout(Duration::from_secs(2))
crates/slskr/src/daemon_runtime_shutdown.rs:46:        let elapsed = shutdown_with_timeout(runtime, timeout);
crates/slskr/src/daemon_runtime_shutdown.rs:59:            .recv_timeout(Duration::from_secs(2))
crates/slskr/src/soulfind_bridge_runtime.rs:24:    tokio::time::timeout(BRIDGE_READ_TIMEOUT, bridge_read_frame_inner(stream))
crates/slskr/src/soulfind_bridge_runtime.rs:30:pub(super) async fn bridge_read_frame_with_timeout(
crates/slskr/src/soulfind_bridge_runtime.rs:34:    tokio::time::timeout(timeout_duration, bridge_read_frame_inner(stream))
crates/slskr/src/soulfind_bridge_runtime.rs:70:    tokio::time::timeout(
crates/slskr/src/soulfind_bridge_runtime.rs:296:        let joined = time::timeout(MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, async {
crates/slskr/src/feature_mutation_controller.rs:344:            .timeout(solid.timeout)
crates/slskr/src/mesh_gateway_controller.rs:213:    let reply = match time::timeout(
crates/slskr/src/share_backfill_controller.rs:140:            let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/controller_reload.rs:61:        let mut interval = time::interval(Duration::from_millis(200));
crates/slskr/src/event_runtime.rs:95:        let loki_result = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/songid_runtime.rs:87:        .timeout(Duration::from_secs(10))
crates/slskr/src/songid_runtime.rs:115:    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
crates/slskr/src/songid_runtime.rs:262:        .timeout(Duration::from_secs(20))
crates/slskr/src/songid_runtime.rs:340:        if let Some(metadata) = tokio::time::timeout(
crates/slskr/src/hash_backfill_runtime.rs:185:            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
crates/slskr/src/hash_backfill_runtime.rs:205:    let received_token = time::timeout(
crates/slskr/src/hash_backfill_runtime.rs:215:    time::timeout(
crates/slskr/src/hash_backfill_runtime.rs:224:    time::timeout(
crates/slskr/src/hash_backfill_runtime.rs:356:        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
crates/slskr/src/mesh_dht_runtime.rs:14:        let mut interval = time::interval(Duration::from_secs(30 * 60));
crates/slskr/src/mesh_dht_runtime.rs:165:    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
crates/slskr/src/mesh_dht_runtime.rs:175:    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
crates/slskr/src/legacy_route_dispatch_group_03.rs:354:            let interests = match time::timeout(
crates/slskr/src/external_visualizer_processes.rs:91:        let result = tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/external_visualizer_processes.rs:173:        tokio::time::timeout(Duration::from_secs(2), async {
crates/slskr/src/daemon_serve.rs:102:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/daemon_serve.rs:891:        let mut interval = tokio::time::interval(Duration::from_secs(1));
crates/slskr/src/daemon_serve.rs:1152:                        let _ = time::timeout(
crates/slskr/src/daemon_serve.rs:1161:                        let _ = time::timeout(
crates/slskr/src/daemon_serve.rs:1171:                        let _ = time::timeout(
crates/slskr/src/lifecycle_controller.rs:33:    let _ = time::timeout(
crates/slskr/src/legacy_route_dispatch_group_04.rs:213:                tokio::spawn(async move {
crates/slskr/src/peer_transport.rs:146:    let stream = time::timeout(settings.timeout_connect, async {
crates/slskr/src/peer_transport.rs:348:                    Ok(stream) => time::timeout(
crates/slskr/src/peer_transport.rs:386:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:414:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:440:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:594:    time::timeout(
crates/slskr/src/peer_transport.rs:601:    let message = time::timeout(
crates/slskr/src/peer_transport.rs:622:    time::timeout(
crates/slskr/src/peer_transport.rs:633:    let message = time::timeout(
crates/slskr/src/peer_transport.rs:652:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:660:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/peer_transport.rs:664:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:680:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:688:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/peer_transport.rs:692:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:773:                let stream = time::timeout(
crates/slskr/src/peer_transport.rs:785:                time::timeout(
crates/slskr/src/peer_transport.rs:793:                let stream = time::timeout(
crates/slskr/src/peer_transport.rs:801:                time::timeout(
crates/slskr/src/peer_transport.rs:928:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:936:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/peer_transport.rs:940:    time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:956:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:964:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/peer_transport.rs:968:    time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:985:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:993:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/peer_transport.rs:997:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:1015:    let stream = time::timeout(
crates/slskr/src/peer_transport.rs:1023:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/peer_transport.rs:1027:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/peer_transport.rs:1059:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/activitypub_controller.rs:204:        .timeout(std::time::Duration::from_secs(5))
crates/slskr/src/route_dispatch_group_1_discovery.rs:501:                let response = tokio::time::timeout(
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:13:        Self(tokio::spawn(future))
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:74:        time::timeout(Duration::from_secs(2), ready_rx)
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:86:                assert!(time::timeout(Duration::from_millis(20), task)
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:92:            time::timeout(Duration::from_secs(2), closed)
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:116:        time::timeout(Duration::from_secs(2), ready_rx)
crates/slskr/src/cli_smoke_soak_owners/probe_task.rs:122:        time::timeout(Duration::from_secs(2), closed)
crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs:106:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs:125:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs:137:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs:224:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli_smoke_soak_owners/peer_smoke_scenarios.rs:112:    match time::timeout(Duration::from_secs(2), TcpStream::connect(address)).await {
crates/slskr/src/cli_smoke_soak_owners/peer_smoke_scenarios.rs:175:    let result = time::timeout(timeout, peer.receive())
crates/slskr/src/cli_smoke_soak_owners/peer_smoke_scenarios.rs:181:    time::timeout(timeout, server_task)
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:22:                time::timeout(send_timeout, session.send_ping())
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:33:                    time::timeout(
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:53:            match time::timeout(next_wait, session.receive()).await {
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:190:                match time::timeout(timeout, handle_live_soak_connect_to_peer_response(response))
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:267:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:287:            let peer_response = match time::timeout(remaining, peer.receive()).await {
crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs:329:        let token = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli_smoke_soak_owners/peer_connection_helpers.rs:42:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli_smoke_soak_owners/peer_connection_helpers.rs:58:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:11:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:53:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:148:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:210:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:249:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:293:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/server_probe_helpers.rs:332:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/live_task_set.rs:107:        let owner = tokio::spawn(async move {
crates/slskr/src/cli_smoke_soak_owners/live_task_set.rs:120:            tokio::time::timeout(std::time::Duration::from_secs(1), peer.read(&mut [0]))
crates/slskr/src/cli_smoke_soak_owners/live_task_set.rs:135:            let owner = tokio::spawn(async move {
crates/slskr/src/cli_smoke_soak_owners/live_task_set.rs:160:            time::timeout(Duration::from_secs(2), owner)
crates/slskr/src/cli_smoke_soak_owners/live_task_set.rs:166:                time::timeout(Duration::from_secs(1), client.read(&mut [0]))
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:69:    time::timeout(timeout, server_task)
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:102:        let received = time::timeout(timeout, parent_peer.receive())
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:134:    let received = time::timeout(timeout, second_peer.receive())
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:141:    if time::timeout(Duration::from_millis(25), first_peer.receive())
crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs:248:        match time::timeout(remaining, first.receive()).await {
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:9:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:57:        match time::timeout(
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:103:            match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:154:            let message = time::timeout(Duration::from_secs(5), distributed.receive())
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:171:            let token = time::timeout(Duration::from_secs(5), transfer.receive_token())
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:190:        match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs:239:    match time::timeout(Duration::from_secs(5), peer.receive_user_info_request()).await {
crates/slskr/src/cli_smoke_soak_owners/peer_probe_operations.rs:249:    let stream = time::timeout(timeout, TcpStream::connect(connect_address.as_str()))
crates/slskr/src/cli_smoke_soak_owners/peer_probe_operations.rs:258:    let (incoming, _) = time::timeout(timeout, accept_task)
crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs:56:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs:170:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs:189:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs:201:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/config_parts/environment_layers.rs:3:pub(super) fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
crates/slskr/src/file_transfer_runtime_owners/inbound_transfer.rs:38:        let remote_token = time::timeout(
crates/slskr/src/file_transfer_runtime_owners/upload_streaming.rs:60:        time::timeout(
crates/slskr/src/file_transfer_runtime_owners/upload_streaming.rs:68:    let offset = time::timeout(
crates/slskr/src/file_transfer_runtime_owners/upload_streaming.rs:108:        time::timeout(
crates/slskr/src/file_transfer_runtime_owners/download_progress.rs:11:    let token = time::timeout(
crates/slskr/src/file_transfer_runtime_owners/download_progress.rs:24:    time::timeout(
crates/slskr/src/file_transfer_runtime_owners/download_progress.rs:40:        let chunk = time::timeout(
crates/slskr/src/config_parts/startup.rs:158:        let reconnect_delay = validated_runtime_interval(
crates/slskr/src/config_parts/startup.rs:167:        let ping_interval = validated_runtime_interval(
crates/slskr/src/config_parts/startup.rs:360:        let peer_response_timeout = validated_runtime_interval(
crates/slskr/src/session_runtime_owners/wishlist_dispatch.rs:23:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/session_runtime_owners/session_supervision.rs:116:                    wishlist_scheduler.set_server_interval(server_interval);
crates/slskr/src/session_runtime_owners/session_supervision.rs:143:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/session_runtime_owners/session_supervision.rs:194:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/session_runtime_owners/session_supervision.rs:197:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/session_runtime_owners/session_supervision.rs:201:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/config_file_parts/peer_transport.rs:201:        let timeout_connect = parse_timeout(
crates/slskr/src/config_file_parts/peer_transport.rs:212:        let timeout_inactivity = parse_timeout(
crates/slskr/src/config_file_parts/peer_transport.rs:227:        let timeout_transfer = parse_timeout(
crates/slskr/src/file_transfer_runtime_owners/audio_metadata.rs:70:    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
crates/slskr/src/file_transfer_runtime_owners/audio_metadata.rs:141:        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
crates/slskr/src/session_runtime_owners/peer_connection.rs:60:    time::timeout(
crates/slskr/src/private_gateway_owners/peer_authentication.rs:25:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway_owners/peer_authentication.rs:38:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway_owners/quic_relay.rs:22:pub(super) async fn read_quic_data_command_line_with_timeout(
crates/slskr/src/private_gateway_owners/quic_relay.rs:25:    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
crates/slskr/src/private_gateway_owners/quic_relay.rs:34:    timeout(DESTINATION_WRITE_TIMEOUT, async {
crates/slskr/src/private_gateway_owners/shared_quic_runtime.rs:102:        let mut sweep = tokio::time::interval(Duration::from_secs(1));
crates/slskr/src/private_gateway_owners/tests.rs:36:    let reader_task = tokio::spawn(async move {
crates/slskr/src/private_gateway_owners/tests.rs:52:    let result = timeout(Duration::from_secs(1), reader_task)
crates/slskr/src/private_gateway_owners/tests.rs:297:    let forwarder = tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway_owners/tests.rs:305:    let (size, source) = tokio::time::timeout(
crates/slskr/src/private_gateway_owners/tests.rs:675:            timeout(Duration::from_secs(2), backend.recv_from(&mut packet))
crates/slskr/src/private_gateway_owners/tests.rs:682:            timeout(Duration::from_secs(2), async {
crates/slskr/src/private_gateway_owners/tests.rs:741:    let (client, accepted) = timeout(Duration::from_secs(2), async {
crates/slskr/src/private_gateway_owners/tests.rs:779:        timeout(Duration::from_secs(2), client.read(&mut byte))
crates/slskr/src/private_gateway_owners/gateway_transport.rs:720:                match timeout(QUIC_DATA_READ_TIMEOUT, connection.accept_inbound_stream()).await {
crates/slskr/src/private_gateway_owners/gateway_transport.rs:746:                        match timeout(QUIC_DATA_READ_TIMEOUT, receive.read_to_end()).await {
crates/slskr/src/private_gateway_owners/gateway_transport.rs:779:        let (line, line_bytes) = match read_quic_data_command_line_with_timeout(&mut receive).await
crates/slskr/src/private_gateway_owners/gateway_transport.rs:802:            let relay_line = match read_quic_data_command_line_with_timeout(&mut receive).await {
crates/slskr/src/private_gateway_owners/gateway_transport.rs:840:                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
crates/slskr/src/private_gateway_owners/gateway_transport.rs:848:            if timeout(DESTINATION_WRITE_TIMEOUT, send.write_all(b"OK\n"))
crates/slskr/src/private_gateway_owners/gateway_transport.rs:863:            match timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await {
crates/slskr/src/private_gateway_owners/gateway_transport.rs:879:        let remaining = match timeout(
crates/slskr/src/private_gateway_owners/gateway_transport.rs:923:                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
crates/slskr/src/private_gateway_owners/gateway_services.rs:12:        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
crates/slskr/src/private_gateway_owners/gateway_services.rs:23:        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
crates/slskr/src/private_gateway_owners/gateway_services.rs:96:                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
crates/slskr/src/private_gateway_owners/gateway_services.rs:214:        let search = timeout(Duration::from_secs(5), async {
crates/slskr/src/private_gateway_owners/gateway_services.rs:437:        let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway_owners/gateway_services.rs:659:        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
crates/slskr/src/private_gateway_owners/gateway_services.rs:720:        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
web/scripts/capture-readme-screenshots.mjs:311:  window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/capture-readme-screenshots.mjs:312:  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');
web/scripts/audit-react-webui.mjs:640:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:641:      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
web/scripts/audit-react-webui.mjs:642:      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
web/scripts/audit-react-webui.mjs:644:        window.localStorage.setItem(
web/src/lib/communityQualitySignals.js:41:    return window.localStorage;
web/src/lib/session.js:18:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:31:  setToken(sessionStorage, token);
web/src/components/Browse/Browse.jsx:15:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:37:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:110:  // Save tabs to localStorage whenever they change
web/src/components/Search/Detail/SearchDetail.jsx:248:  // Sync hasSavedDefault across tabs/searches when localStorage changes
web/src/lib/storage.js:5:    const value = window.localStorage.getItem(key);
web/src/lib/storage.js:16:    window.localStorage.setItem(key, value);
web/src/lib/storage.js:27:    window.localStorage.removeItem(key);
web/src/lib/storage.js:39:      { length: window.localStorage.length },
web/src/lib/storage.js:40:      (_, index) => window.localStorage.key(index),
web/src/lib/storage.js:51:    const value = window.sessionStorage.getItem(key);
web/src/lib/storage.js:62:    window.sessionStorage.setItem(key, value);
web/src/lib/storage.js:82:    window.sessionStorage.removeItem(key);
web/src/components/Rooms/Rooms.jsx:55:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:65:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:155:  // Save tabs to localStorage whenever they change
web/src/components/Shared/Footer.jsx:223:              target="_blank"
web/src/components/Shared/Footer.jsx:257:              target="_blank"
web/src/components/Shared/Footer.jsx:327:                target="_blank"
web/src/components/Shared/Footer.jsx:346:                  target="_blank"
web/src/components/Shared/Footer.jsx:356:                target="_blank"
web/src/components/Chat/Chat.jsx:46:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:56:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:215:  // Save tabs to localStorage whenever they change
web/src/lib/searches.js:90:// Blocked users management (localStorage-based)
web/src/lib/safeOpen.js:22:    const opened = window.open(url, '_blank', 'noopener,noreferrer');

## Go SDK transport, decoding, and filesystem boundaries
client-go/errors.go:86:	if err := json.Unmarshal(body, &payload); err != nil {
client-go/websocket.go:143:		cancelDial()
client-go/websocket.go:169:			conn, err = baseNetDialContext(dialCtx, network, address)
client-go/websocket.go:171:			conn, err = baseNetDial(network, address)
client-go/websocket.go:173:			conn, err = (&net.Dialer{}).DialContext(dialCtx, network, address)
client-go/websocket.go:189:	conn, _, err := dialer.DialContext(dialContext, w.url, headers)
client-go/websocket.go:190:	cancelDial()
client-go/websocket.go:277:	parsed, err := url.Parse(baseURL)
client-go/websocket.go:718:	err := json.Unmarshal(data, &result)
client-go/client.go:30:	HTTPClient *http.Client
client-go/client.go:41:		HTTPClient: &http.Client{},
client-go/client.go:47:	parsed, err := url.Parse(baseURL)
client-go/client.go:790:	httpClient := http.Client{}
client-go/client.go:798:	resp, err := httpClient.Do(req)
client-go/client.go:843:		if err := json.Unmarshal(raw, &fields); err != nil {
client-go/client.go:857:		if err := json.Unmarshal(raw, &values); err != nil {
client-go/client.go:871:		if err := json.Unmarshal(raw, &result); err != nil {
client-go/client.go:877:		if err := json.Unmarshal(raw, &result); err != nil {
client-go/client.go:888:		if err := json.Unmarshal(raw, &result); err != nil {
client-go/client.go:964:	body, err := io.ReadAll(io.LimitReader(resp.Body, maximum+1))
client-go/client.go:980:	if err := json.Unmarshal(body, &decoded); err == nil {
client-go/client.go:989:	if err := json.Unmarshal(body, &decodedList); err == nil {

## Python SDK transport, decoding, and filesystem boundaries
client-python/slskr/websocket.py:28:        parsed_url = urlsplit(base_url)
client-python/slskr/websocket.py:40:        self.url = urlunsplit(
client-python/slskr/websocket.py:48:        self.session: Optional[aiohttp.ClientSession] = None
client-python/slskr/websocket.py:95:                session = aiohttp.ClientSession()
client-python/slskr/websocket.py:98:                self.ws = await asyncio.wait_for(
client-python/slskr/websocket.py:99:                    session.ws_connect(
client-python/slskr/websocket.py:107:                    await asyncio.wait_for(
client-python/slskr/websocket.py:121:                self._message_task = asyncio.create_task(self._handle_messages(self.ws))
client-python/slskr/websocket.py:145:            await asyncio.gather(connecting_task, return_exceptions=True)
client-python/slskr/websocket.py:185:        self._reconnect_task = asyncio.create_task(self._reconnect_loop())
client-python/slskr/websocket.py:223:            await asyncio.gather(*outbound_tasks, return_exceptions=True)
client-python/slskr/websocket.py:242:            event = json.loads(data)
client-python/slskr/websocket.py:288:        task = asyncio.create_task(ws.send_json(message))
client-python/slskr/websocket.py:375:            task = asyncio.create_task(self._call_listener(listener, argument))
client-python/slskr/client.py:36:        parsed_url = urlsplit(base_url)
client-python/slskr/client.py:46:        self.base_url = urlunsplit(
client-python/slskr/client.py:54:        self.session: Optional[aiohttp.ClientSession] = None
client-python/slskr/client.py:73:            self.session = aiohttp.ClientSession()
client-python/slskr/client.py:583:            url += "?" + urlencode(params)
client-python/slskr/client.py:596:            async with session.request(
client-python/slskr/client.py:686:        async for chunk in response.content.iter_chunked(64 * 1024):
client-python/slskr/client.py:694:        return json.loads(body)
client-python/slskr/client.py:697:        return quote(str(value), safe="")

## Suppressed CI and script failures
.github/workflows/release-publish.yml:282:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:404:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:443:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:486:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:557:        continue-on-error: true
.github/workflows/release-publish.yml:599:        continue-on-error: true
.github/workflows/react-nightly-audit.yml:38:          trap 'sudo systemctl stop "$unit" >/dev/null 2>&1 || true' EXIT INT TERM
scripts/verify-release-artifacts.sh:80:host_target="$(rustc -Vv 2>/dev/null | sed -n 's/^host: //p' | head -n 1 || true)"
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/start-proton-listener-soak.sh:21:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:22:sudo wg-quick down "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:23:sudo ip link del "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:24:sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/start-proton-listener-soak.sh:25:sudo ip netns del "$namespace" 2>/dev/null || true
scripts/test-certification-phases.sh:85:  set +e
scripts/run-in-proton-wg-netns.sh:37:    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:38:    sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:39:    sudo rm -rf "/etc/netns/$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:40:    sudo ip link del "$host_veth" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:42:        sudo ip route del "$endpoint_ip/32" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:44:    sudo iptables -t nat -D POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:45:    sudo iptables -D FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:46:    sudo iptables -D FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:139:sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
scripts/scan-bug-council-candidates.sh:82:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/with-process-memory-guard.sh:68:    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:8:  docker rm -f "$container_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:22:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:35:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:41:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:48:state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:51:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:58:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:64:  docker logs "$container_name" 2>&1 || true
scripts/probe-natpmp-mapping.sh:33:            "$collision_private_port" tcp 0 >/dev/null 2>&1 || true
scripts/probe-natpmp-mapping.sh:37:            "$private_port" tcp 0 >/dev/null 2>&1 || true
scripts/validate-changelog.sh:15:unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
scripts/run-cross-client-validation.sh:89:  set +e
scripts/run-cross-client-validation.sh:93:  detail="$( { tail -n 40 "$stdout_file"; grep -E '^(error:|FAILED|Failed|Build FAILED|Test Run Failed|warning |thread |panicked|Unhandled exception)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:169:  set +e
scripts/run-cross-client-validation.sh:249:  set +e
scripts/run-cross-client-validation.sh:305:    health="$(curl -fsS --max-time 2 "$health_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:306:    app="$(curl -fsS --max-time 2 "$app_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:443:    set +e
scripts/run-cross-client-validation.sh:451:    detail="$( { cat "$stdout_file"; grep -E '^(error:|thread |panicked|failed|rejected)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:476:    kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:477:    wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:495:  wait_for_daemon_preflight "$scope" "$name" "$daemon_host" "$http_port" || true
scripts/run-cross-client-validation.sh:511:      kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:512:      wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:564:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55130 || true
scripts/run-cross-client-validation.sh:586:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55131 || true
scripts/run-council-active-bughunt.sh:104:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/run-certification.sh:155:        set +e
scripts/run-certification.sh:283:    grep -oE 'port=[0-9]+ obfuscation_type=[0-9]+ obfuscated_port=[0-9]+' <<<"$1" | tail -1 || true
scripts/run-certification.sh:304:        set +e
scripts/run-certification.sh:364:    set +e
scripts/run-certification.sh:381:    set +e
scripts/run-certification.sh:399:    set +e
scripts/run-certification.sh:418:        server_ip="$(getent ahostsv4 vps.slsknet.org 2>/dev/null | awk 'NR == 1 { print $1 }')" || true
scripts/run-certification.sh:452:            set +e
scripts/run-certification.sh:541:    set +e
scripts/run-certification.sh:569:    set +e
scripts/run-certification.sh:591:    set +e
scripts/run-certification.sh:613:    set +e
scripts/run-certification.sh:649:    tmux kill-session -t "$listener_session" 2>/dev/null || true
scripts/run-certification.sh:693:    set +e
scripts/run-certification.sh:727:    set +e
scripts/run-certification.sh:760:    set +e
scripts/run-certification.sh:792:    set +e
scripts/run-certification.sh:819:    set +e
scripts/run-certification.sh:878:    set +e
scripts/run-certification.sh:907:    set +e
scripts/run-certification.sh:932:    set +e
scripts/run-certification.sh:949:    set +e
scripts/run-certification.sh:975:    set +e
scripts/run-certification.sh:1013:    set +e
scripts/run-certification.sh:1045:    set +e
scripts/run-certification.sh:1085:    set +e
scripts/run-certification.sh:1114:    set +e
scripts/run-certification.sh:1141:    set +e
scripts/run-certification.sh:1168:    set +e
scripts/run-certification.sh:1198:        set +e
scripts/run-certification.sh:1208:            set +e
scripts/run-certification.sh:1283:    set +e
scripts/run-certification.sh:1312:    set +e
scripts/run-certification.sh:1346:    set +e
scripts/run-certification.sh:1358:        set +e
scripts/run-certification.sh:1396:            set +e
scripts/run-certification.sh:1436:    set +e
scripts/run-certification.sh:1468:        set +e
scripts/run-certification.sh:1499:        set +e
scripts/run-certification.sh:1530:    set +e
scripts/run-certification.sh:1545:    set +e
scripts/run-certification.sh:1562:        set +e
scripts/run-certification.sh:1589:    set +e
scripts/run-certification.sh:1614:    set +e
scripts/generate-vpn-soulseek-accounts.sh:98:  set +e
scripts/run-live-interop-matrix.sh:86:  live_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }' || true)"
scripts/run-live-interop-matrix.sh:154:    tail -n 20 "$stderr_file" || true
scripts/run-live-interop-matrix.sh:171:  set +e
scripts/run-live-interop-matrix.sh:201:set +e
scripts/run-live-interop-matrix.sh:227:set +e
scripts/run-live-interop-matrix.sh:248:set +e
scripts/check-controller-auth-profiles.sh:20:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:21:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:34:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:70:  tail -80 "$work_dir/$target.log" >&2 || true
scripts/check-aur-package-smoke.sh:41:    IFS= read -r container_id < "$container_id_file" || true
scripts/check-aur-package-smoke.sh:43:      docker rm --force "$container_id" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:37:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:40:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
scripts/run-live-soak-proton-natpmp.sh:65:        renew_ports_once || true
scripts/run-live-soak-proton-natpmp.sh:75:        kill "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:76:        wait "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:80:            >/dev/null 2>&1 || true
scripts/run-live-soak-proton-natpmp.sh:84:            >/dev/null 2>&1 || true
scripts/check-rust-format.sh:68:    diff -u -- "$rust_file" "$formatted_file" || true
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:134:  latest_tag="$(git tag --sort=-creatordate --list 'build-main-*' | head -n 1 || true)"
scripts/check-local-identity-leaks.sh:136:    latest_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
scripts/check-web-cors-differential.sh:20:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:21:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:123:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:135:      tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:140:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:346:    tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:350:  wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:353:    tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:23:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:24:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:105:      tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:110:  tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:293:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:300:  tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:24:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:25:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:114:      tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:119:  tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:23:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:26:        wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:32:    kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:33:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:111:      tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:116:  tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:296:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:303:  tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:17:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:18:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:84:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:100:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:120:      tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:125:  tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:134:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:141:  tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:19:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:20:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:97:      tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:102:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:41:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:42:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:108:      tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:113:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:288:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
scripts/check-web-auth-credentials-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:39:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:40:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:116:      tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:121:  tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:525:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/run-slskdn-cross-client-interop.sh:198:' 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:205:  set +e
scripts/run-slskdn-cross-client-interop.sh:258:  grep -cF -- "$needle" "$slskdn_log" 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:381:slskdn_binary="$(discover_slskdn_binary || true)"
scripts/run-slskdn-cross-client-interop.sh:391:  slskdn_binary="$(discover_slskdn_binary || true)"
scripts/run-slskdn-cross-client-interop.sh:510:      if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:518:  tail -n 120 "$slskr_log" >&2 || true
scripts/run-slskdn-cross-client-interop.sh:555:      kill "$pid" 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:556:      wait "$pid" 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:678:      distributed_parent_target_state="$(curl -sS "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:731:      if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
scripts/run-slskdn-cross-client-interop.sh:739:  tail -n 120 "$slskdn_log" >&2 || true
scripts/run-slskdn-cross-client-interop.sh:748:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:750:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:752:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" || true
scripts/run-slskdn-cross-client-interop.sh:754:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/options" || true
scripts/run-slskdn-cross-client-interop.sh:756:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/run-slskdn-cross-client-interop.sh:759:try_request slskr-share-rescan auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" '{}' >/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:762:  || true
scripts/run-slskdn-cross-client-interop.sh:809:  if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:825:  if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
scripts/run-slskdn-cross-client-interop.sh:913:    intent_id="$(printf '%s' "$response_body" | json_get desiredTrackId 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:957:    release_id="$(printf '%s' "$release_body" | json_get desiredReleaseId 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:976:    process_track_id="$(printf '%s' "$process_body" | json_get desiredTrackId 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:991:      process_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$process_track_id")" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:992:      process_status="$(printf '%s' "$process_body" | json_get status 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1023:    artist_id="$(printf '%s' "$artists_body" | json_get '0.artistId' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1026:      release_group_id="$(printf '%s' "$release_body" | json_get '0.releaseGroupId' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1031:      positive_track_id="$(printf '%s' "$tracks_body" | json_get '0.trackId' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1053:        "track=$positive_track_id status=$(printf '%s' "$response_body" | json_get status 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1060:      positive_intent_id="$(printf '%s' "$response_body" | json_get desiredTrackId 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1078:        process_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$positive_intent_id")" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1079:        positive_status="$(printf '%s' "$process_body" | json_get status 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1087:          "track=$positive_track_id status=Completed source=$(printf '%s' "$process_body" | json_get plannedSources 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1164:    before_obfuscated_accepts="$(printf '%s' "$before_listeners" | json_get obfuscated_accepts 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1165:    before_obfuscated_messages="$(printf '%s' "$before_listeners" | json_get obfuscated_peer_messages 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1178:    after_obfuscated_accepts="$(printf '%s' "$after_listeners" | json_get obfuscated_accepts 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1179:    after_obfuscated_messages="$(printf '%s' "$after_listeners" | json_get obfuscated_peer_messages 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1252:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:1253:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:1355:    "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/sync/$escaped_slskr" '{}' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1357:    "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/sync/$escaped_slskr" '{}' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1359:    "http://127.0.0.1:$slskr_http_port/api/v0/mesh/sync/$escaped_slskdn" '{}' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1361:    "http://127.0.0.1:$slskr_http_port/api/v0/mesh/sync/$escaped_slskdn" '{}' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1391:      "$slskdn_overlay_port" || true
scripts/run-slskdn-cross-client-interop.sh:1892:probe_peer_address slskr "$slskr_username" || true
scripts/run-slskdn-cross-client-interop.sh:1893:probe_peer_address slskdn "$slskdn_username" || true
scripts/run-slskdn-cross-client-interop.sh:1911:  target_user_status="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/status" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1912:  target_user_info="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/info" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1932:      distributed_target_state="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1933:      if [[ "$(printf '%s' "$distributed_target_state" | json_get distributedNetwork.canAcceptChildren 2>/dev/null || true)" == "true" ]]; then
scripts/run-slskdn-cross-client-interop.sh:1962:' 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1992:    distributed_target_state="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:1993:    distributed_reverse_state="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:2008:      && [[ "$(printf '%s' "$distributed_reverse_state" | json_get distributedNetwork.branchLevel 2>/dev/null || true)" =~ ^[1-9][0-9]*$ ]]; then
scripts/run-slskdn-cross-client-interop.sh:2037:    status="$(printf '%s' "$transfer_json" | json_get status 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:2038:    bytes="$(printf '%s' "$transfer_json" | json_get bytes_transferred 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:2063:  success="$(printf '%s' "$response" | json_get success 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:2064:  hash="$(printf '%s' "$response" | json_get hash 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:2091:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:2093:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:2095:    auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/check-web-audit.sh:28:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-audit.sh:40:    ' <<<"$report" 2>/dev/null || true
scripts/check-web-audit.sh:54:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:35:      kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:37:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:93:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:107:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:108:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:122:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$slskd_report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:123:  tail -80 "$log_file" >&2 || true
scripts/run-slskd-api-compat-smoke.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:30:    wait "$daemon_pid" 2>/dev/null || true
scripts/run-live-http-transfer-smoke.sh:152:      kill "$pid" 2>/dev/null || true
scripts/run-live-http-transfer-smoke.sh:153:      wait "$pid" 2>/dev/null || true
scripts/run-live-http-transfer-smoke.sh:220:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-live-http-transfer-smoke.sh:228:  tail -n 80 "$work_dir/$name.log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:243:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" && "${seen:-0}" -ge 6 ]]; then
scripts/run-live-http-transfer-smoke.sh:268:      regular_local_addr="$(printf '%s' "$listeners" | json_field regular_local_addr 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:309:  tail -n 40 "$stdout_file" >&2 || true
scripts/run-live-http-transfer-smoke.sh:310:  tail -n 40 "$stderr_file" >&2 || true
scripts/run-live-http-transfer-smoke.sh:318:    auth_post_json "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse/request" '{}' >/dev/null || true
scripts/run-live-http-transfer-smoke.sh:322:        status="$(printf '%s' "$browse_json" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:323:        count="$(printf '%s' "$browse_json" | json_field count 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:325:          count="$(printf '%s' "$browse_json" | json_field fileCount 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:343:  tail -n 80 "$target_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:367:  status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:368:  bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:374:    tail -n 80 "$source_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:375:    tail -n 80 "$target_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:381:status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:382:bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:385:  tail -n 80 "$source_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:386:  tail -n 80 "$target_log" >&2 || true
scripts/run-proton-public-matrix.sh:222:    set +e
scripts/run-proton-public-matrix.sh:302:    set +e
scripts/run-proton-public-matrix.sh:328:                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
scripts/run-proton-public-matrix.sh:334:                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
scripts/run-proton-public-matrix.sh:421:    wait_for_metadata "$listener" "$metadata_probe" || true
scripts/check-controller-options-differential.sh:89:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:90:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:100:    kill "$soulseek_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:101:    wait "$soulseek_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:108:    kill "$listener_blocker_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:109:    wait "$listener_blocker_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:116:    kill "$lidarr_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:117:    wait "$lidarr_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:128:    git -C "$upstream_repo" worktree remove --force "$slskd_root" >/dev/null 2>&1 || true
scripts/check-controller-options-differential.sh:131:    git -C "$upstream_repo" worktree remove --force "$slskdn_root" >/dev/null 2>&1 || true
scripts/check-controller-options-differential.sh:171:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:177:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:193:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:199:  tail -120 "$log" >&2 || true
scripts/options_differential/part_06.sh:247:    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
scripts/options_differential/part_01.sh:647:    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
scripts/options_differential/part_01.sh:686:      | "$python_bin" -c 'import json,sys; values=json.load(sys.stdin)["shares"]["directories"]; print(values[0] if values else "")' 2>/dev/null || true)"
scripts/options_differential/part_01.sh:692:      tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:698:  tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:958:      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["directories"]["downloads"])' 2>/dev/null || true)"
scripts/options_differential/part_01.sh:962:      tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:968:  tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1090:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteFileManagement"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_01.sh:1094:      tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1100:  tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1246:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteConfiguration"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_01.sh:1250:      tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1256:  tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1432:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["debug"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_01.sh:1436:      tail -120 "$log" >&2 || true
scripts/options_differential/part_01.sh:1442:  tail -120 "$log" >&2 || true
scripts/run-slskd-cross-client-interop.sh:145:' "$query" 2>/dev/null || true
scripts/run-slskd-cross-client-interop.sh:165:      kill "$pid" 2>/dev/null || true
scripts/run-slskd-cross-client-interop.sh:166:      wait "$pid" 2>/dev/null || true
scripts/run-slskd-cross-client-interop.sh:206:  target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:246:  target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:247:  rust_state="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/session" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:267:  rust_distributed_state="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:279:  "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" >/dev/null 2>&1 || true
scripts/run-slskd-cross-client-interop.sh:282:  rust_share_catalog="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/shares/catalog?q=commons-click-track.ogg" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:321:target_user_status="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$escaped_slskr_username/status" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:322:target_user_info="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$escaped_slskr_username/info" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:334:    distributed_target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:364:  distributed_target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:365:  distributed_reverse_state="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:419:target_browse="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/browse" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:428:  "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/directory" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:438:rust_browse="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:451:  rust_folder_status="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse/status" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:452:  rust_folder_entries="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:468:set +e
scripts/run-slskd-cross-client-interop.sh:491:  "http://127.0.0.1:$slskd_http_port/api/v0/searches" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:492:target_search_id="$(printf '%s' "$target_search_created" | json_field id 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:497:    target_search_body="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/searches/$target_search_id?includeResponses=true" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:515:  "http://127.0.0.1:$slskd_http_port/api/v0/conversations/$slskr_username" || true)"
scripts/run-slskd-cross-client-interop.sh:517:rust_messages="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/messages/$slskd_username" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:528:  "http://127.0.0.1:$slskr_http_port/api/v0/messages" || true)"
scripts/run-slskd-cross-client-interop.sh:530:target_messages="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/conversations/$slskr_username/messages" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:538:  kill "$slskd_pid" 2>/dev/null || true
scripts/run-slskd-cross-client-interop.sh:539:  wait "$slskd_pid" 2>/dev/null || true
scripts/run-slskd-cross-client-interop.sh:564:  restart_target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:575:restart_browse="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/browse" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:583:  "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/directory" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:593:  "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined" || true)"
scripts/run-slskd-cross-client-interop.sh:595:  "http://127.0.0.1:$slskr_http_port/api/v0/rooms/$room_name/join" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:600:  "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined/$room_name/messages" || true)"
scripts/run-slskd-cross-client-interop.sh:603:  rust_room_messages="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room_name/messages" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:617:  "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room_name/messages" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:620:  target_room_messages="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined/$room_name/messages" 2>/dev/null || true)"
scripts/run-slskd-cross-client-interop.sh:634:  set +e
scripts/run-slskd-cross-client-interop.sh:659:  "http://127.0.0.1:$slskd_http_port/api/v0/transfers/downloads/$slskr_username" || true)"
scripts/options_differential/part_05.sh:18:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:24:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:147:      cat "$log" >&2 || true
scripts/options_differential/part_05.sh:184:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:190:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:211:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:217:  cat "$status" >&2 || true
scripts/options_differential/part_05.sh:218:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:226:    curl --silent --max-time 30 "$base_url/api/v0/users/fixture-peer/browse" >/dev/null || true
scripts/options_differential/part_05.sh:478:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:605:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:611:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:633:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:639:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:841:      tail -120 "$conflict_log" >&2 || true
scripts/options_differential/part_05.sh:929:      tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:936:  tail -120 "$log" >&2 || true
scripts/options_differential/part_05.sh:1391:      cat "$fixture_log" >&2 || true
scripts/options_differential/part_04.sh:76:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:82:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:97:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:103:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:115:      count="$(sqlite3 "$state/data/transfers.db" 'SELECT COUNT(*) FROM Transfers;' 2>/dev/null || true)"
scripts/options_differential/part_04.sh:117:      count="$($python_bin - "$state/transfer-state.json" 2>/dev/null <<'PY' || true
scripts/options_differential/part_04.sh:126:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:132:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:154:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:160:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:401:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:407:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:424:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:430:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:781:        tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:788:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:918:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_04.sh:923:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_04.sh:1016:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_04.sh:1110:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_04.sh:1123:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_04.sh:1128:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1199:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1206:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1235:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1242:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1360:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1366:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1396:      tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1402:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1427:  cat "$status" >&2 || true
scripts/options_differential/part_04.sh:1428:  tail -120 "$log" >&2 || true
scripts/options_differential/part_04.sh:1442:    actual="$(advertisement_count "$status" 2>/dev/null || true)"
scripts/options_differential/part_04.sh:1445:      actual="$(advertisement_count "$status" 2>/dev/null || true)"
scripts/options_differential/part_04.sh:1454:  cat "$status" >&2 || true
scripts/options_differential/part_04.sh:1455:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:143:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:150:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:161:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["pendingReconnect"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_02.sh:166:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:173:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:411:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:418:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:443:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:450:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:643:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConfigWatch"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_02.sh:647:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:653:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:745:      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["soulseek"]["description"])' 2>/dev/null || true)"
scripts/options_differential/part_02.sh:749:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:755:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:779:      tail -120 "$daemon_log" >&2 || true
scripts/options_differential/part_02.sh:785:  cat "$suite/$label.error" >&2 || true
scripts/options_differential/part_02.sh:786:  tail -120 "$daemon_log" >&2 || true
scripts/options_differential/part_02.sh:878:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConnect"]).lower())' 2>/dev/null || true)"
scripts/options_differential/part_02.sh:882:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:888:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:905:      cat "$log" >&2 || true
scripts/options_differential/part_02.sh:936:  cat "$status" >&2 || true
scripts/options_differential/part_02.sh:937:  tail -120 "$daemon_log" >&2 || true
scripts/options_differential/part_02.sh:952:    cat "$status" >&2 || true
scripts/options_differential/part_02.sh:953:    tail -120 "$daemon_log" >&2 || true
scripts/options_differential/part_02.sh:1068:        tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1075:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1152:      cat "$fixture_status" >&2 || true
scripts/options_differential/part_02.sh:1245:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1251:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1426:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1432:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1601:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1607:  tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1760:      tail -120 "$log" >&2 || true
scripts/options_differential/part_02.sh:1766:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:14:    tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:149:      tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:155:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:315:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_03.sh:451:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_03.sh:590:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/options_differential/part_03.sh:1077:      tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1083:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1276:      tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1282:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1476:      tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1482:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1637:      cat "$log" >&2 || true
scripts/options_differential/part_03.sh:1661:      tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1667:  tail -120 "$log" >&2 || true
scripts/options_differential/part_03.sh:1699:      tail -120 "$daemon_log" >&2 || true
scripts/options_differential/part_03.sh:1705:  tail -120 "$daemon_log" >&2 || true
