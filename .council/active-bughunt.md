# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Protocol-controlled allocations and lengths
crates/slskr-client/src/transfer.rs:208:            connection.read_chunk(remaining).await
crates/slskr-client/src/quic_data.rs:554:    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
crates/slskr-client/src/capabilities.rs:173:        let mut features = Vec::with_capacity(feature_count);
crates/slskr-client/src/capabilities.rs:596:    String::from_utf8(reader.read_bytes(length)?.to_vec())
crates/slskr-client/src/capabilities.rs:617:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:668:    let mut output = Vec::with_capacity(values.len());
crates/slskr-client/src/mesh_sync.rs:432:        let mut output = Vec::with_capacity(encoded.len());
crates/slskr-client/src/mesh_sync.rs:1030:            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
crates/slskr-client/src/overlay.rs:212:        let mut payload = vec![0_u8; length];
crates/slskr-client/src/overlay.rs:1270:        let mut payload = vec![0; 15];
crates/slskr-client/src/overlay.rs:1501:        let mut signature = vec![0_u8; 64];
crates/slskr-client/src/quic_control.rs:41:    let mut encoded = Vec::with_capacity(key_value_len + 5);
crates/slskr-client/src/io.rs:203:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:298:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:358:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:389:    let mut obfuscated = Vec::with_capacity(encoded_len);
crates/slskr-client/src/file_transfer.rs:108:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:127:        let mut chunk = vec![0; length];
crates/slskr-client/src/file_transfer.rs:147:        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
crates/slskr-client/src/file_transfer.rs:168:        let mut payload = Vec::with_capacity(length);
crates/slskr-client/src/file_transfer.rs:192:        let mut encoded = Vec::with_capacity(first_block.len() + length);
crates/slskr-client/src/overlay_control.rs:77:        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
crates/slskr-client/src/overlay_control.rs:111:        let payload = reader.read_bytes("payload")?;
crates/slskr-client/src/overlay_control.rs:357:    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
crates/slskr-client/src/search.rs:562:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr-protocol/src/distributed.rs:114:                    payload: reader.read_bytes(reader.remaining())?.to_vec(),
crates/slskr-client/src/listener.rs:240:        let mut encoded = Vec::with_capacity(4 + candidate_length);
crates/slskr-client/src/listener.rs:268:    let mut obfuscated = Vec::with_capacity(8 + length);
crates/slskr-client/src/listener.rs:380:            let mut nested = Vec::with_capacity(nested_len);
crates/slskr-protocol/src/primitives.rs:107:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:133:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:134:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:142:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:159:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:192:            output: Vec::with_capacity(capacity),
crates/slskr-protocol/src/peer.rs:727:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-protocol/src/peer.rs:740:        let compressed = compress_zlib(&vec![b'x'; MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES + 1])
crates/slskr-protocol/src/server.rs:1220:                let payload = reader.read_bytes(reader.remaining())?.to_vec();
crates/slskr-protocol/src/server.rs:2129:    let mut usernames = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2135:    let mut statuses = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2141:    let mut data = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2152:    let mut slots = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2158:    let mut countries = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2163:    let mut users = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2251:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2286:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2341:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2380:    let counts_len = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/server.rs:2389:    let mut entries = Vec::with_capacity(names.len());
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr-web/src/lib.rs:17772:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/lib.rs:17773:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr/src/utils.rs:713:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:731:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:1063:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr/src/bloom_filter.rs:39:            bits: vec![0_u8; bit_size.div_ceil(8)],
crates/slskr/src/content_discovery.rs:236:        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
crates/slskr/src/content_discovery.rs:245:        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
crates/slskr/src/content_discovery.rs:359:        let mut normalized = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:632:        let mut valid = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:643:        let mut candidates = Vec::with_capacity(valid.len());
crates/slskr/src/content_discovery.rs:795:    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
crates/slskr/src/content_discovery.rs:887:    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:916:    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
crates/slskr/src/relay_ws.rs:407:    let mut header = Vec::with_capacity(10);
crates/slskr/src/relay_ws.rs:487:    let mut payload = vec![0_u8; length as usize];
crates/slskr/src/quic_alpn.rs:172:    let mut output = vec![0_u8; length];
crates/slskr/src/quic_alpn.rs:185:    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
crates/slskr/src/search_fallback.rs:37:    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
crates/slskr/src/route_dispatch.rs:82:    let mut normalized = Vec::with_capacity(terms.len());
crates/slskr/src/route_dispatch_group_4.rs:1834:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/mesh_sync.rs:116:            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
crates/slskr/src/mesh_sync.rs:228:    let mut incoming = Vec::with_capacity(received);
crates/slskr/src/mesh_sync.rs:298:async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
crates/slskr/src/mesh_sync.rs:352:    let mut data = vec![0_u8; to_read];
crates/slskr/src/relay.rs:1247:        let mut quotient = Vec::with_capacity(source.len());
crates/slskr/src/config.rs:9883:    let mut peers = Vec::with_capacity(values.len());
crates/slskr/src/relay_agent.rs:597:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay_agent.rs:748:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/events_ws.rs:258:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:356:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:537:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/events_ws.rs:713:        let payload = vec![b'x'; 1024 * 1024];
crates/slskr/src/route_dispatch_group_2.rs:1805:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/private_gateway.rs:1134:            let mut response = vec![0_u8; 65_536];
crates/slskr/src/private_gateway.rs:1324:    let mut bytes = Vec::with_capacity(256);
crates/slskr/src/private_gateway.rs:1327:        let read = receive.read_chunk(&mut byte).await?;
crates/slskr/src/private_gateway.rs:1403:            .read_chunk(&mut buffer[..remaining])
crates/slskr/src/private_gateway.rs:1851:            let mut bytes = vec![0_u8; length];
crates/slskr/src/private_gateway.rs:2096:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/private_gateway.rs:2967:        let mut packet = vec![0_u8; 1_200];
crates/slskr/src/private_gateway.rs:3167:            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
crates/slskr/src/multisource.rs:480:        let mut sources = Vec::with_capacity(request.sources.len());
crates/slskr/src/multisource.rs:522:        let mut source_busy = vec![false; sources.len()];
crates/slskr/src/multisource.rs:526:        let mut results = Vec::with_capacity(chunks.len());
crates/slskr/src/multisource.rs:760:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/port_forwarding.rs:282:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/port_forwarding.rs:742:            data: vec![7; TUNNEL_CHUNK_BYTES],
crates/slskr/src/port_forwarding.rs:752:            data: vec![7; TUNNEL_CHUNK_BYTES + 1],
crates/slskr/src/dotnet_regex.rs:309:    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
crates/slskr/src/dotnet_regex.rs:325:    let mut named_slots = Vec::with_capacity(named.len());
crates/slskr/src/dotnet_regex.rs:347:    let mut targets = vec![String::new(); maximum_slot + 1];
crates/slskr/src/security_controls.rs:1819:        let mut transformed = Vec::with_capacity(bucket + 4);
crates/slskr/src/http_server.rs:453:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:557:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/http_server.rs:922:        let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/http_server.rs:1078:        let body = vec![b'x'; 100 * 1024];
crates/slskr/src/webhooks.rs:1350:        let mut persisted = vec![invalid; MAX_WEBHOOKS];
crates/slskr/src/cli.rs:1120:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1347:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:2897:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:3209:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:3660:        .read_chunk(5)
crates/slskr/src/lib.rs:6456:            let mut bytes = Vec::with_capacity(33);
crates/slskr/src/lib.rs:10233:        let mut updated = Vec::with_capacity(distinct_ids.len());
crates/slskr/src/lib.rs:14237:    let mut items = Vec::with_capacity(candidates.len());
crates/slskr/src/lib.rs:15454:        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:15456:            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
crates/slskr/src/lib.rs:15458:        "url" => vec!["URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:23751:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/lib.rs:28162:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/lib.rs:36680:    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
crates/slskr/src/lib.rs:46252:        let mut records = Vec::with_capacity(raw_records.len());
crates/slskr/src/lib.rs:48298:    let mut events = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:48667:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/lib.rs:49066:    let mut requested_files = Vec::with_capacity(files.len());
crates/slskr/src/lib.rs:54437:    let mut payload = vec![0_u8; length - 4];
crates/slskr/src/lib.rs:54528:    let mut provided_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:54529:    let mut configured_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:55554:    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
crates/slskr/src/lib.rs:55664:    let mut lines = Vec::with_capacity(parsed.headers.len());
crates/slskr/src/lib.rs:62138:            let mut results = Vec::with_capacity(work.len());
crates/slskr/src/lib.rs:63590:        let mut current = Vec::with_capacity(right.len() + 1);
crates/slskr/src/lib.rs:64369:        let mut results = Vec::with_capacity(descriptors.len());
crates/slskr/src/lib.rs:64513:        let mut results = Vec::with_capacity(ids.len());
crates/slskr/src/lib.rs:67762:                let mut peers = Vec::with_capacity(peer_records.len());
crates/slskr/src/lib.rs:68294:                let mut entries = Vec::with_capacity(requests.len());
crates/slskr/src/lib.rs:77488:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:81557:        connection.read_chunk(wanted),
crates/slskr/src/lib.rs:82125:    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
crates/slskr/src/lib.rs:82421:    let mut buffer = vec![0_u8; state.config.soulseek_connection.buffer_transfer];
crates/slskr/src/lib.rs:82880:            connection.read_chunk(next_len),
crates/slskr/src/lib.rs:83032:    let mut order = Vec::with_capacity(2);
crates/slskr/src/lib.rs:83225:            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
crates/slskr/src/lib.rs:83304:    let mut bound_address_and_port = vec![0_u8; address_len + 2];
crates/slskr/src/controller_tests.rs:814:        vec![0; 12]
crates/slskr/src/controller_tests.rs:2741:        let chunk = vec![b' '; 64 * 1024];
crates/slskr/src/controller_tests.rs:2787:                let chunk = vec![b'x'; 64 * 1024];
crates/slskr/src/controller_tests.rs:8725:    let mut attribute = Vec::with_capacity(8);
crates/slskr/src/controller_tests.rs:8731:    let mut response = Vec::with_capacity(32);
crates/slskr/src/controller_tests.rs:19121:        record.results = vec![template.clone(); super::MAX_SEARCH_RESULTS_PER_SEARCH];
crates/slskr/src/controller_tests.rs:21547:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:21923:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22028:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22114:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22279:    assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
crates/slskr/src/controller_tests.rs:24079:        record.members = vec![template.clone(); super::MAX_SHARE_GROUP_MEMBERS];
crates/slskr/src/controller_tests.rs:24241:        record.items = vec![template.clone(); super::MAX_COLLECTION_ITEMS];
crates/slskr/src/controller_tests.rs:28585:        let mut frame = Vec::with_capacity(4 + length as usize);
crates/slskr/src/controller_tests.rs:28700:            let mut actual = vec![0_u8; expected.len()];
crates/slskr/src/controller_tests.rs:103171:        vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103491:        vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103551:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103593:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103749:            let mut bytes = vec![0_u8; 65_536];
crates/slskr/src/controller_tests.rs:117856:        vec![0_u8; 64 * 1024 + 1],
crates/slskr/src/controller_tests.rs:119796:    let low = entropy.check(&vec![0_u8; EntropyControl::SAMPLE_SIZE]);

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/route_dispatch_group_7.rs:2153:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/route_dispatch_group_7.rs:2357:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/application_state.rs:43:        "forwardedPort": runtime.vpn.forwarded_port,
crates/slskr/src/private_gateway.rs:272:    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
crates/slskr/src/private_gateway.rs:3000:        .expect("DHT response should be forwarded")
crates/slskr/src/vpn.rs:15:    pub forwarded_port: Option<u16>,
crates/slskr/src/vpn.rs:148:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:167:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:185:    client: &reqwest::Client,
crates/slskr/src/vpn.rs:210:    let client = reqwest::Client::builder()
crates/slskr/src/vpn.rs:211:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/vpn.rs:250:    let mut forwarded_port = None;
crates/slskr/src/vpn.rs:254:        forwarded_port = primary
crates/slskr/src/vpn.rs:290:                if forwarded_port.is_none() {
crates/slskr/src/vpn.rs:291:                    forwarded_port = port_forwards
crates/slskr/src/vpn.rs:301:        is_ready: !options.port_forwarding || forwarded_port.is_some(),
crates/slskr/src/vpn.rs:305:        forwarded_port,
crates/slskr/src/vpn.rs:409:        assert_eq!(status.forwarded_port, Some(44_444));
crates/slskr/src/vpn.rs:445:        assert_eq!(status.forwarded_port, Some(55_555));
crates/slskr/src/vpn.rs:512:        assert_eq!(status.forwarded_port, Some(45_678));
crates/slskr/src/relay_agent.rs:231:) -> Result<reqwest::Client, String> {
crates/slskr/src/relay_agent.rs:232:    let mut builder = reqwest::Client::builder()
crates/slskr/src/relay_agent.rs:233:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/relay_agent.rs:563:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:629:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:729:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:773:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:806:    client: &reqwest::Client,
crates/slskr/src/http_server.rs:67:    pub forwarded: Option<String>,
crates/slskr/src/http_server.rs:68:    pub x_forwarded_for: Option<String>,
crates/slskr/src/http_server.rs:122:                    "forwarded" => headers.forwarded = Some(value.to_string()),
crates/slskr/src/http_server.rs:123:                    "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
crates/slskr/src/http_server.rs:379:            "forwarded" => append_list_header(&mut headers.forwarded, value),
crates/slskr/src/http_server.rs:380:            "x-forwarded-for" => append_list_header(&mut headers.x_forwarded_for, value),
crates/slskr/src/http_server.rs:1040:            headers.forwarded,
crates/slskr/src/http_server.rs:1044:            headers.x_forwarded_for,
crates/slskr/src/http_server.rs:1241:            request.headers.x_forwarded_for.as_deref(),
crates/slskr/src/http_server.rs:1245:            request.headers.forwarded.as_deref(),
crates/slskr/src/cli.rs:2499:    let forwarded = tree
crates/slskr/src/cli.rs:2503:    if forwarded != 1 {
crates/slskr/src/cli.rs:2505:            "distributed search reached {forwarded} children instead of one"
crates/slskr/src/port_forwarding.rs:84:                "Port {} is already being forwarded",
crates/slskr/src/port_forwarding.rs:98:            bytes_forwarded: Arc::new(AtomicU64::new(0)),
crates/slskr/src/port_forwarding.rs:155:    bytes_forwarded: Arc<AtomicU64>,
crates/slskr/src/port_forwarding.rs:280:        let send_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:297:        let receive_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:331:        let bytes_forwarded = self.bytes_forwarded.load(Ordering::Relaxed);
crates/slskr/src/port_forwarding.rs:340:            bytes_forwarded,
crates/slskr/src/port_forwarding.rs:343:            performance: Performance::new(active_connections, bytes_forwarded),
crates/slskr/src/port_forwarding.rs:514:    pub bytes_forwarded: u64,
crates/slskr/src/port_forwarding.rs:870:                if status.bytes_forwarded == 10 {
crates/slskr/src/multisource.rs:656:    let mut builder = Client::builder()
crates/slskr/src/multisource.rs:657:        .redirect(Policy::none())
crates/slskr/src/multisource.rs:661:        builder = builder.resolve(host, SocketAddr::new(address.ip(), port));
crates/slskr/src/webhooks.rs:579:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:580:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:759:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:760:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:763:            client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:15161:        .to_socket_addrs()
crates/slskr/src/lib.rs:15171:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15173:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:15341:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15343:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:36103:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/lib.rs:36310:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:37413:        let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:37414:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:42115:                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
crates/slskr/src/lib.rs:43898:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:43900:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:43919:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:43921:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44153:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44155:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44207:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44209:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44835:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44837:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45109:        .to_socket_addrs()
crates/slskr/src/lib.rs:45130:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45132:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45169:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45171:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45200:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45202:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45227:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45229:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46047:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46049:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46052:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46089:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46091:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46094:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46849:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46851:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46854:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46983:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46985:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46988:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47616:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47618:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47621:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47792:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:47794:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48148:                .to_socket_addrs()
crates/slskr/src/lib.rs:48165:        .to_socket_addrs()
crates/slskr/src/lib.rs:48202:        .to_socket_addrs()
crates/slskr/src/lib.rs:48524:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/lib.rs:48529:fn forwarded_client_ip(
crates/slskr/src/lib.rs:48534:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/lib.rs:48535:        forwarded_header_client_ips(value)?
crates/slskr/src/lib.rs:48537:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/lib.rs:48538:        x_forwarded_for_client_ips(value)?
crates/slskr/src/lib.rs:48541:    forwarded_ips
crates/slskr/src/lib.rs:48553:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:48556:        .map(parse_forwarded_ip_token)
crates/slskr/src/lib.rs:48561:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:48564:        .map(parse_forwarded_element_ip)
crates/slskr/src/lib.rs:48569:fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:48570:    let mut forwarded_ip = None;
crates/slskr/src/lib.rs:48576:        if forwarded_ip.is_some() {
crates/slskr/src/lib.rs:48579:        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
crates/slskr/src/lib.rs:48581:    forwarded_ip
crates/slskr/src/lib.rs:48584:fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:55691:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:55693:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:55695:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:56849:        let client = match reqwest::Client::builder()
crates/slskr/src/lib.rs:56851:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:71637:        reqwest::Client::new().post(endpoint).json(&payload).send(),
crates/slskr/src/lib.rs:73056:                    "primary" => status.forwarded_port,
crates/slskr/src/lib.rs:73093:/// VPN's forwarded port. The local listener remains bound to the configured
crates/slskr/src/lib.rs:73100:            .forwarded_port
crates/slskr/src/lib.rs:84307:            reqwest::Client::new().post(endpoint).json(&payload).send(),
crates/slskr/src/controller_tests.rs:2876:fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
crates/slskr/src/controller_tests.rs:2886:        x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
crates/slskr/src/controller_tests.rs:2891:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3362:fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
crates/slskr/src/controller_tests.rs:3368:        forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
crates/slskr/src/controller_tests.rs:3373:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3379:fn forwarded_ip_parser_rejects_malformed_authorities() {
crates/slskr/src/controller_tests.rs:3392:            super::parse_forwarded_ip_token(malformed),
crates/slskr/src/controller_tests.rs:3398:        super::parse_forwarded_ip_token("\"[2001:db8::42]:443\""),
crates/slskr/src/controller_tests.rs:3402:        super::parse_forwarded_ip_token("198.51.100.24:443"),
crates/slskr/src/controller_tests.rs:3409:fn forwarded_elements_require_one_valid_for_parameter() {
crates/slskr/src/controller_tests.rs:3412:        super::parse_forwarded_element_ip("proto=https; for=198.51.100.24; by=10.0.0.2"),
crates/slskr/src/controller_tests.rs:3423:            super::parse_forwarded_element_ip(malformed),
crates/slskr/src/controller_tests.rs:3438:        x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
crates/slskr/src/controller_tests.rs:3443:        .expect("forwarded client address");
crates/slskr/src/controller_tests.rs:3459:        x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
crates/slskr/src/controller_tests.rs:3470:fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
crates/slskr/src/controller_tests.rs:3476:        forwarded: Some("for=unknown".to_owned()),
crates/slskr/src/controller_tests.rs:3477:        x_forwarded_for: Some("203.0.113.99".to_owned()),
crates/slskr/src/controller_tests.rs:6214:        forwarded_port: Some(44_444),
crates/slskr/src/controller_tests.rs:6237:            "forwardedPort": 44444,
crates/slskr/src/controller_tests.rs:93504:            forwarded_port: Some(44_499),
crates/slskr/src/controller_tests.rs:93529:                && application["vpn"]["forwardedPort"] == 44_499
crates/slskr/src/controller_tests.rs:99025:        let client = reqwest::Client::new();

## Filesystem and persistent-state boundaries
crates/slskr/src/pod_channels.rs:371:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pod_channels.rs:466:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:488:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:497:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:522:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:531:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:556:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:565:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:589:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/mesh_services.rs:57:            let _ = std::fs::remove_file(&self.path);
crates/slskr/src/mesh_services.rs:288:    let mut options = tokio::fs::OpenOptions::new();
crates/slskr/src/mesh_services.rs:506:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:535:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:548:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:591:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:601:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:626:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/mesh_services.rs:649:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/mesh_services.rs:690:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/realm_subject_index.rs:347:            fs::create_dir_all(parent)
crates/slskr/src/realm_subject_index.rs:353:        fs::rename(&temporary, path)
crates/slskr/src/ftp.rs:674:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:698:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:705:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:728:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:736:            tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:768:            tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:781:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:794:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:806:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:813:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:834:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:873:        tokio::fs::remove_file(file).await.unwrap();
crates/slskr/src/storage.rs:106:    OpenOptions::new()
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
crates/slskr/src/content_discovery.rs:945:    let mut options = fs::OpenOptions::new();
crates/slskr/src/content_discovery.rs:1285:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1309:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/scripts.rs:97:    tokio::fs::create_dir_all(script_directory)
crates/slskr/src/scripts.rs:230:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:254:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:311:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/pods.rs:1402:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pods.rs:1555:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1568:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1577:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1583:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1623:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1652:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1661:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1684:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1693:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1725:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1734:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1759:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1768:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1833:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1842:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1855:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1864:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1889:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/multisource.rs:72:            let _ = fs::remove_file(&self.path);
crates/slskr/src/multisource.rs:94:        let _ = fs::remove_dir_all(&self.path);
crates/slskr/src/multisource.rs:474:    fs::create_dir_all(parent).map_err(|_| "output directory could not be created".to_owned())?;
crates/slskr/src/multisource.rs:603:        let _ = fs::remove_file(&assembly_path);
crates/slskr/src/multisource.rs:792:    fs::remove_file(assembly_path)
crates/slskr/src/multisource.rs:811:    let mut options = fs::OpenOptions::new();
crates/slskr/src/multisource.rs:1142:        fs::remove_dir_all(root).expect("remove permissions test root");
crates/slskr/src/multisource.rs:1214:        fs::remove_dir_all(root).expect("remove swarm test root");
crates/slskr/src/multisource.rs:1283:        fs::remove_dir_all(root).expect("remove swarm cancellation test root");
crates/slskr/src/multisource.rs:1312:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/multisource.rs:1373:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/relay.rs:1214:    fs::rename(&temporary_path, &manifest_path)
crates/slskr/src/relay.rs:1417:            tokio::fs::remove_file(path)
crates/slskr/src/relay.rs:1428:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1459:        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
crates/slskr/src/relay.rs:1469:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1491:        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
crates/slskr/src/mesh_security.rs:1044:                fs::create_dir_all(&mesh_directory)
crates/slskr/src/mesh_security.rs:1205:        if let Err(error) = fs::rename(&temporary, &self.storage_path) {
crates/slskr/src/mesh_security.rs:1206:            let _ = fs::remove_file(&temporary);
crates/slskr/src/focused_controller_tests.rs:42:    fs::create_dir_all(&state_dir).expect("create focused test state directory");
crates/slskr/src/focused_controller_tests.rs:236:    fs::create_dir_all(&evidence_dir).expect("create focused evidence directory");
crates/slskr/src/focused_controller_tests.rs:248:    fs::create_dir_all(&evidence_dir).expect("create focused file evidence directory");
crates/slskr/src/focused_controller_tests.rs:285:    fs::create_dir_all(downloads.join("Artist/Album")).expect("downloads fixture");
crates/slskr/src/focused_controller_tests.rs:287:    fs::create_dir_all(incomplete.join("Partial")).expect("incomplete fixture");
crates/slskr/src/focused_controller_tests.rs:383:        fs::create_dir_all(path.parent().expect("delete fixture parent"))
crates/slskr/src/focused_controller_tests.rs:386:            fs::create_dir_all(&path).expect("delete fixture directory path");
crates/slskr/src/focused_controller_tests.rs:609:            fs::create_dir_all(&target_dir).expect("create storage list target");
crates/slskr/src/focused_controller_tests.rs:638:            let _ = fs::remove_file(link);
crates/slskr/src/focused_controller_tests.rs:639:            let _ = fs::remove_dir_all(target_dir);
crates/slskr/src/focused_controller_tests.rs:679:    let _ = fs::remove_file(downloads_conflict);
crates/slskr/src/focused_controller_tests.rs:680:    let _ = fs::remove_file(incomplete_conflict);
crates/slskr/src/focused_controller_tests.rs:1019:    fs::create_dir_all(managed_file.parent().expect("managed file parent"))
crates/slskr/src/focused_controller_tests.rs:1276:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/focused_controller_tests.rs:1280:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/focused_controller_tests.rs:1411:    fs::create_dir_all(root.join("legacy")).expect("create legacy profile root");
crates/slskr/src/focused_controller_tests.rs:1412:    fs::create_dir_all(root.join("native")).expect("create native profile root");
crates/slskr/src/focused_controller_tests.rs:1451:    let _ = fs::remove_dir_all(root);
crates/slskr/src/relay_agent.rs:587:    fs::create_dir_all(&relay_directory)
crates/slskr/src/relay_agent.rs:621:    let _ = fs::remove_file(&database_path).await;
crates/slskr/src/relay_agent.rs:864:            let _ = fs::remove_file(&temporary).await;
crates/slskr/src/relay_agent.rs:877:    fs::rename(&temporary, &destination)
crates/slskr/src/relay_agent.rs:905:            let _ = std::fs::remove_file(&self.path);
crates/slskr/src/private_gateway.rs:2662:    fs::create_dir_all(state_dir)
crates/slskr/src/private_gateway.rs:2688:        return match fs::remove_file(certificate_path) {
crates/slskr/src/private_gateway.rs:2717:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2760:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2772:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2777:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2780:    if let Err(error) = fs::remove_file(&temporary) {
crates/slskr/src/private_gateway.rs:2798:        fs::create_dir_all(&path).unwrap();
crates/slskr/src/private_gateway.rs:3095:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3121:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3150:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3159:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3173:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3188:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3199:        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
crates/slskr/src/private_gateway.rs:3204:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3220:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3234:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3253:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/http_server.rs:1727:        std::fs::remove_file(path).unwrap();
crates/slskr/src/http_server.rs:1769:        std::fs::remove_file(path).unwrap();
crates/slskr/src/persistence.rs:21:    let file = OpenOptions::new()
crates/slskr/src/persistence.rs:34:    file.set_permissions(std::fs::Permissions::from_mode(0o600))
crates/slskr/src/persistence.rs:5627:        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();
crates/slskr/src/persistence.rs:5643:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/route_dispatch_group_2.rs:2138:                    let _ = fs::remove_file(path);
crates/slskr/src/route_dispatch_group_2.rs:2186:                    let _ = fs::remove_file(path);
crates/slskr/src/config.rs:1974:    let file = fs::OpenOptions::new()
crates/slskr/src/config.rs:1980:    fs::remove_file(&probe).map_err(|_| format!("{field} writeability probe cleanup failed"))?;
crates/slskr/src/config.rs:8479:    let mut options = fs::OpenOptions::new();
crates/slskr/src/config.rs:9419:    let mut options = fs::OpenOptions::new();
crates/slskr/src/config.rs:11543:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11559:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11602:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11631:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11679:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11704:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11751:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11779:        std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config.rs:11792:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11836:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11855:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11868:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11895:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11982:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12019:        std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config.rs:12038:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12091:            std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12108:            std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12119:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12138:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12228:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12283:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12605:        std::fs::create_dir_all(&yaml_downloads).unwrap();
crates/slskr/src/config.rs:12606:        std::fs::create_dir_all(&yaml_incomplete).unwrap();
crates/slskr/src/config.rs:12607:        std::fs::create_dir_all(&yaml_share_a).unwrap();
crates/slskr/src/config.rs:12608:        std::fs::create_dir_all(&yaml_share_b).unwrap();
crates/slskr/src/config.rs:12609:        std::fs::create_dir_all(&env_downloads).unwrap();
crates/slskr/src/config.rs:12671:        std::fs::create_dir_all(&relative_root).unwrap();
crates/slskr/src/config.rs:12701:        std::fs::remove_dir_all(relative_root).unwrap();
crates/slskr/src/config.rs:12702:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12715:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12758:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12771:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12856:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12869:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12906:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12919:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12960:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12973:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13022:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13036:        std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/config.rs:13056:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13069:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13091:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13107:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13118:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13119:        let _ = std::fs::remove_file(outside);
crates/slskr/src/config.rs:13140:        let _ = std::fs::remove_file(path);
crates/slskr/src/config.rs:13159:        let _ = std::fs::remove_dir(path);
crates/slskr/src/config.rs:13175:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13184:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13202:        let _ = std::fs::remove_file(path);
crates/slskr/src/config.rs:13773:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13807:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:13820:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13847:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14469:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14519:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14584:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14585:        std::fs::create_dir_all(&content).unwrap();
crates/slskr/src/config.rs:14682:        std::fs::remove_dir_all(&content).unwrap();
crates/slskr/src/config.rs:14683:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14710:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14779:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14808:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14979:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14988:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15003:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:15073:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15205:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:15214:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15229:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/lib.rs:6440:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6466:                file.set_permissions(fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:6474:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6488:            fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:12311:        let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:13992:            types: canonicalize(
crates/slskr/src/lib.rs:14005:            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
crates/slskr/src/lib.rs:14006:            statuses: canonicalize(
crates/slskr/src/lib.rs:15320:    let _ = fs::remove_file(&normalized_path);
crates/slskr/src/lib.rs:15758:    match (path.canonicalize(), root.canonicalize()) {
crates/slskr/src/lib.rs:16070:                match (normalized.canonicalize(), root.canonicalize()) {
crates/slskr/src/lib.rs:16110:    let writable = fs::OpenOptions::new()
crates/slskr/src/lib.rs:16116:        let _ = fs::remove_file(probe);
crates/slskr/src/lib.rs:16728:            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
crates/slskr/src/lib.rs:17406:            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
crates/slskr/src/lib.rs:18430:        fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:24032:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:24063:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:36701:        .canonicalize()
crates/slskr/src/lib.rs:36730:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/lib.rs:36753:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/lib.rs:36859:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:36906:    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:36907:    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:39502:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:42309:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:43785:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:43878:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:43886:    match fs::remove_file(path) {
crates/slskr/src/lib.rs:46945:    let directory = fs::canonicalize(directory)
crates/slskr/src/lib.rs:46953:        fs::remove_file(&path).map_err(|error| {
crates/slskr/src/lib.rs:50454:                                    let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:50460:                            let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:50476:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:69857:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:69874:            .canonicalize()
crates/slskr/src/lib.rs:69881:                .canonicalize()
crates/slskr/src/lib.rs:69886:                .canonicalize()
crates/slskr/src/lib.rs:71569:        fs::remove_file(path)
crates/slskr/src/lib.rs:71573:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:71578:    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
crates/slskr/src/lib.rs:72689:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/lib.rs:72701:        std::fs::create_dir_all(path)
crates/slskr/src/lib.rs:72748:    std::fs::create_dir_all(path).map_err(|error| {
crates/slskr/src/lib.rs:73892:    let _ = fs::remove_file(output_path);
crates/slskr/src/lib.rs:76721:        let canonical_path = local_path.canonicalize().ok()?;
crates/slskr/src/lib.rs:76725:            .filter_map(|root| root.canonicalize().ok())
crates/slskr/src/lib.rs:76741:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:76930:        let _ = fs::remove_file(&uploaded.path);
crates/slskr/src/lib.rs:77027:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:77032:        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
crates/slskr/src/lib.rs:77050:    let file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:77058:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77123:                let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77135:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77434:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:77441:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:78028:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:78036:            .canonicalize()
crates/slskr/src/lib.rs:78038:        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
crates/slskr/src/lib.rs:78058:            fs::remove_dir_all(&path)
crates/slskr/src/lib.rs:78064:            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/lib.rs:78204:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/lib.rs:78212:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:78216:        .canonicalize()
crates/slskr/src/lib.rs:78221:        .canonicalize()
crates/slskr/src/lib.rs:78313:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:78364:        .canonicalize()
crates/slskr/src/lib.rs:78367:        .canonicalize()
crates/slskr/src/lib.rs:78372:    fs::OpenOptions::new()
crates/slskr/src/lib.rs:81963:        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
crates/slskr/src/lib.rs:81975:            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
crates/slskr/src/lib.rs:82608:            fs::OpenOptions::new()
crates/slskr/src/lib.rs:82674:        fs::rename(&final_path, &incomplete_path)
crates/slskr/src/lib.rs:82702:        fs::remove_file(&completed_path)
crates/slskr/src/lib.rs:82705:    match fs::rename(&incomplete_path, &completed_path) {
crates/slskr/src/lib.rs:82713:            fs::remove_file(&incomplete_path)
crates/slskr/src/lib.rs:82825:        fs::create_dir_all(&root)
crates/slskr/src/lib.rs:82832:        fs::rename(path, destination)
crates/slskr/src/lib.rs:82835:        fs::remove_file(path)
crates/slskr/src/lib.rs:84277:        if tokio::fs::create_dir_all(&log_dir).await.is_ok() {
crates/slskr/src/lib.rs:84278:            if let Ok(mut file) = tokio::fs::OpenOptions::new()
crates/slskr/src/lib.rs:86879:            let _ = fs::remove_dir(&path);
crates/slskr/src/lib.rs:86882:            let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:86989:                let _ = fs::remove_file(entry.path());
crates/slskr/src/lib.rs:88036:                let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:89647:        match root.canonicalize() {
crates/slskr/src/lib.rs:89736:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/lib.rs:89999:        fs::remove_file(&rotated_path)
crates/slskr/src/lib.rs:90002:    fs::rename(path, &rotated_path)
crates/slskr/src/lib.rs:90027:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:90107:    fs::create_dir_all(parent)?;
crates/slskr/src/lib.rs:90132:        let mut file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:90143:            let _ = fs::remove_file(temp_path);
crates/slskr/src/lib.rs:90151:    fs::rename(source, destination)
crates/slskr/src/lib.rs:90159:    match fs::remove_file(destination) {
crates/slskr/src/lib.rs:90164:    fs::rename(source, destination)
crates/slskr/src/lib.rs:90189:    let mut options = fs::OpenOptions::new();
crates/slskr/src/controller_tests.rs:79:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:117:        fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:299:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:334:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:348:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:366:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:380:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:392:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:689:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:713:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:728:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:799:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:805:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:1304:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1397:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:1407:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1455:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1688:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:2313:    fs::create_dir_all(&evidence_dir).expect("create server/session evidence directory");
crates/slskr/src/controller_tests.rs:2676:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:3513:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:4005:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4102:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4376:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4457:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4634:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:5053:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5222:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5325:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5969:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:5992:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6000:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6092:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6100:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6187:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6256:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6320:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6888:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7300:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7327:    std::fs::create_dir_all(&root).expect("gateway state directory");
crates/slskr/src/controller_tests.rs:7891:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7899:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:8716:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:9390:    std::fs::create_dir_all(root.join("assets")).unwrap();
crates/slskr/src/controller_tests.rs:9391:    std::fs::create_dir_all(root.join("static")).unwrap();
crates/slskr/src/controller_tests.rs:9426:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9451:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9460:    std::fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:9469:    let _ = std::fs::remove_file(outside);
crates/slskr/src/controller_tests.rs:9470:    let _ = std::fs::remove_dir_all(outside_dir);
crates/slskr/src/controller_tests.rs:9471:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9492:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9536:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9550:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9567:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9810:    std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:9943:    std::fs::create_dir_all(&album).unwrap();
crates/slskr/src/controller_tests.rs:10037:    std::fs::create_dir_all(&dir).unwrap();
crates/slskr/src/controller_tests.rs:10075:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10076:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:10109:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10145:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10179:        std::fs::create_dir_all(&directory).unwrap();
crates/slskr/src/controller_tests.rs:11640:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:11774:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12176:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12579:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12686:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13033:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13038:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13287:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13292:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14011:    fs::create_dir_all(&evidence_dir).expect("create application evidence directory");
crates/slskr/src/controller_tests.rs:14237:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14381:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14618:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:16692:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19304:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19519:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19892:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20109:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20452:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20534:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20972:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21071:    std::fs::create_dir_all(parent).expect("download parent dir");
crates/slskr/src/controller_tests.rs:21081:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21100:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21101:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21108:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21127:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21128:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21137:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21155:    std::fs::create_dir_all(&dir).expect("test dir");
crates/slskr/src/controller_tests.rs:21161:    std::fs::remove_file(&shared_path).expect("remove shared file");
crates/slskr/src/controller_tests.rs:21171:    let _ = std::fs::remove_dir_all(dir);
crates/slskr/src/controller_tests.rs:21192:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:21193:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:21204:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:21205:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:21244:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21567:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21587:    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
crates/slskr/src/controller_tests.rs:21673:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21747:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21838:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21943:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22046:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22137:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22287:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:26365:    std::fs::create_dir_all(&root).expect("create stream share root");
crates/slskr/src/controller_tests.rs:26433:    std::fs::remove_dir_all(root).expect("remove stream fixture");
crates/slskr/src/controller_tests.rs:26468:    std::fs::create_dir_all(&root).expect("create preview share root");
crates/slskr/src/controller_tests.rs:26532:    std::fs::remove_dir_all(root).expect("remove preview fixture");
crates/slskr/src/controller_tests.rs:26848:    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
crates/slskr/src/controller_tests.rs:26936:    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");
crates/slskr/src/controller_tests.rs:26939:    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
crates/slskr/src/controller_tests.rs:26940:    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
crates/slskr/src/controller_tests.rs:26941:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:27112:    std::fs::create_dir_all(&child).unwrap();
crates/slskr/src/controller_tests.rs:27126:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:27274:    let _ = std::fs::remove_file(&queue.state_path);
crates/slskr/src/controller_tests.rs:27275:    let _ = std::fs::remove_file(&queue.events_path);
crates/slskr/src/controller_tests.rs:27747:    fs::create_dir_all(&root).expect("create overlay search state directory");
crates/slskr/src/controller_tests.rs:27872:    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
crates/slskr/src/controller_tests.rs:27882:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28003:    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
crates/slskr/src/controller_tests.rs:28250:    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
crates/slskr/src/controller_tests.rs:28256:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28555:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:28790:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:28966:    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
crates/slskr/src/controller_tests.rs:29109:    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
crates/slskr/src/controller_tests.rs:29529:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:29705:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:31296:    fs::create_dir_all(&config.downloads_dir).unwrap();
crates/slskr/src/controller_tests.rs:31305:    fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:31316:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:31360:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:31823:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:31969:    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32246:    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
crates/slskr/src/controller_tests.rs:32307:    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
crates/slskr/src/controller_tests.rs:32308:    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32643:    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
crates/slskr/src/controller_tests.rs:32897:    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
crates/slskr/src/controller_tests.rs:33137:    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
crates/slskr/src/controller_tests.rs:33149:    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
crates/slskr/src/controller_tests.rs:33252:    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
crates/slskr/src/controller_tests.rs:34083:    std::fs::create_dir_all(&root).expect("create listening-party share root");
crates/slskr/src/controller_tests.rs:34174:    std::fs::remove_dir_all(root).expect("remove listening-party fixture");
crates/slskr/src/controller_tests.rs:34783:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:34967:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35098:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35410:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35562:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35765:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:36306:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:38940:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39021:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39196:    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
crates/slskr/src/controller_tests.rs:39224:    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
crates/slskr/src/controller_tests.rs:40517:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:40528:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:41812:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:42702:    fs::create_dir_all(root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:42751:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:42878:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:42984:        let _ = fs::remove_file(path);
crates/slskr/src/controller_tests.rs:42987:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:43591:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:43890:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45487:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45591:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:46784:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:46932:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47122:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47338:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47539:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47796:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48089:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48810:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49122:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49507:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49716:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49756:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:49821:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:49827:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50160:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50347:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50721:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:50970:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51433:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52489:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52749:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52895:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53656:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:53945:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:54121:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54295:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54361:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54443:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54512:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54787:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55118:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55585:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55966:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56072:        fs::remove_file(&pods_path).expect("remove channel create state file");
crates/slskr/src/controller_tests.rs:56098:        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
crates/slskr/src/controller_tests.rs:56185:        fs::remove_file(&pods_path).expect("remove channel update state file");
crates/slskr/src/controller_tests.rs:56218:        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
crates/slskr/src/controller_tests.rs:56306:        fs::remove_file(&pods_path).expect("remove channel delete state file");
crates/slskr/src/controller_tests.rs:56332:        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
crates/slskr/src/controller_tests.rs:56410:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:56600:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56839:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56978:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57169:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57368:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57464:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57760:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58302:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58691:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59035:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59476:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59801:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60068:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60187:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60330:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61098:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61335:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61562:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61750:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61843:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61993:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62176:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62457:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62907:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63053:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63230:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:63486:    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
crates/slskr/src/controller_tests.rs:63620:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64044:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:64241:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64615:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64760:    fs::create_dir_all(&evidence_dir).expect("create discovery graph edge evidence directory");
crates/slskr/src/controller_tests.rs:65041:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65286:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65786:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66161:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66489:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67071:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67386:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:67614:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67805:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68178:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68614:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:69067:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:69892:    fs::create_dir_all(&evidence_dir).expect("create quarantine-jury evidence directory");
crates/slskr/src/controller_tests.rs:70131:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:70665:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:71270:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:71551:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72177:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72527:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72968:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73304:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73415:            fs::remove_file(&path).expect("remove message storage file");
crates/slskr/src/controller_tests.rs:73579:        fs::remove_dir(&messages_path).expect("remove blocked global message path");
crates/slskr/src/controller_tests.rs:73731:        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
crates/slskr/src/controller_tests.rs:73757:        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
crates/slskr/src/controller_tests.rs:73788:        fs::remove_dir(&messages_path).expect("remove blocked search message path");
crates/slskr/src/controller_tests.rs:73839:        fs::remove_dir(&messages_path).expect("remove blocked count message path");
crates/slskr/src/controller_tests.rs:73976:            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
crates/slskr/src/controller_tests.rs:73983:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74086:            fs::remove_file(&path).expect("remove membership storage file");
crates/slskr/src/controller_tests.rs:74129:        fs::remove_dir(&pods_path).expect("remove blocked membership delete path");
crates/slskr/src/controller_tests.rs:74218:        fs::remove_dir(&pods_path).expect("remove blocked membership projection path");
crates/slskr/src/controller_tests.rs:74237:        fs::remove_dir(&pods_path).expect("remove blocked membership stats path");
crates/slskr/src/controller_tests.rs:74290:        fs::remove_dir(&pods_path).expect("remove blocked membership moderation path");
crates/slskr/src/controller_tests.rs:74385:        fs::remove_dir(&pods_path).expect("remove blocked membership publish path");
crates/slskr/src/controller_tests.rs:74469:        fs::remove_dir(&pods_path).expect("remove blocked membership update path");
crates/slskr/src/controller_tests.rs:74552:        fs::remove_dir(&pods_path).expect("remove blocked membership cleanup path");
crates/slskr/src/controller_tests.rs:74581:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74649:                fs::remove_file(&path).expect("remove discovery feature state file");
crates/slskr/src/controller_tests.rs:74758:        fs::remove_dir(&feature_path).expect("remove blocked discovery registration path");
crates/slskr/src/controller_tests.rs:74846:        fs::remove_dir(&feature_path).expect("remove blocked discovery update path");
crates/slskr/src/controller_tests.rs:74959:        fs::remove_dir(&feature_path).expect("remove blocked discovery unregister path");
crates/slskr/src/controller_tests.rs:75091:        fs::remove_dir(&feature_path).expect("remove blocked discovery projection path");
crates/slskr/src/controller_tests.rs:75151:        fs::remove_dir(&feature_path).expect("remove blocked discovery refresh path");
crates/slskr/src/controller_tests.rs:75240:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:76060:    fs::create_dir_all(&evidence_dir).expect("create PodJoinLeave evidence directory");
crates/slskr/src/controller_tests.rs:76531:    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
crates/slskr/src/controller_tests.rs:76978:    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
crates/slskr/src/controller_tests.rs:77838:    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
crates/slskr/src/controller_tests.rs:78550:    fs::create_dir_all(&evidence_dir).expect("create MultiSource evidence directory");
crates/slskr/src/controller_tests.rs:78965:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79107:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79363:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79578:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79843:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80070:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80101:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81155:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81414:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:82231:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:82975:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83279:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83539:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:83840:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84045:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84251:                    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:84344:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84462:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84670:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:84675:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84795:    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:84982:    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:84987:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85177:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85521:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85770:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85847:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85945:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86035:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86255:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86434:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86536:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86599:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86669:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86711:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86763:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86818:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87141:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87318:    let _ = fs::remove_file(&validation_path);
crates/slskr/src/controller_tests.rs:87481:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87735:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87867:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:87972:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88153:    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
crates/slskr/src/controller_tests.rs:88372:    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
crates/slskr/src/controller_tests.rs:88532:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88624:    std::fs::create_dir_all(download_file.parent().unwrap())
crates/slskr/src/controller_tests.rs:88682:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:88830:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88916:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89019:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89138:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89190:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89713:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90085:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90154:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90201:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90251:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90305:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90409:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90466:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90527:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90572:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90628:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90685:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90802:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90863:    fs::create_dir_all(&custom_path).expect("create destination fixture");
crates/slskr/src/controller_tests.rs:90920:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:90924:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90979:    fs::create_dir_all(&root).expect("create destination edge root");
crates/slskr/src/controller_tests.rs:91213:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91220:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91460:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:91979:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92702:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92856:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93096:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:93365:        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
crates/slskr/src/controller_tests.rs:93420:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:93426:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93656:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93726:        std::fs::create_dir_all(&root).expect("create differential downloads root");
crates/slskr/src/controller_tests.rs:93757:        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
crates/slskr/src/controller_tests.rs:93808:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94275:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94486:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94589:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95073:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95310:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95471:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96129:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96666:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:97576:    fs::create_dir_all(existing.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:97805:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98296:    fs::create_dir_all(&new_root).unwrap();
crates/slskr/src/controller_tests.rs:98297:    fs::create_dir_all(&new_downloads).unwrap();
crates/slskr/src/controller_tests.rs:98298:    fs::create_dir_all(&new_incomplete).unwrap();
crates/slskr/src/controller_tests.rs:98696:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98727:        fs::create_dir_all(download_file.parent().unwrap()).expect("downloads fixture root");
crates/slskr/src/controller_tests.rs:98728:        fs::create_dir_all(incomplete_file.parent().unwrap()).expect("incomplete fixture root");
crates/slskr/src/controller_tests.rs:98861:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98966:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99184:        let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:99190:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99214:        fs::create_dir_all(&root).expect("secure writer root");
crates/slskr/src/controller_tests.rs:99278:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:99284:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99306:    fs::create_dir_all(&root).expect("DHT certificate root");
crates/slskr/src/controller_tests.rs:99339:        fs::create_dir_all(&linked_root).expect("DHT symlink root");
crates/slskr/src/controller_tests.rs:99397:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99404:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100333:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100358:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100359:    let _ = std::fs::remove_file(&outside);
crates/slskr/src/controller_tests.rs:100384:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100422:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100541:    std::fs::create_dir_all(&nested).expect("create nested dir");
crates/slskr/src/controller_tests.rs:100558:    std::fs::create_dir_all(&album).expect("create recursive directory");
crates/slskr/src/controller_tests.rs:100567:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100588:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100589:    std::fs::create_dir_all(&outside).expect("create outside");
crates/slskr/src/controller_tests.rs:100602:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100603:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:100620:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100635:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100655:    std::fs::create_dir_all(&directory).expect("create deep directory tree");
crates/slskr/src/controller_tests.rs:100665:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101362:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101369:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101383:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101389:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101448:    std::fs::create_dir_all(&artist).unwrap();
crates/slskr/src/controller_tests.rs:101450:    std::fs::create_dir_all(root.join(".hidden")).unwrap();
crates/slskr/src/controller_tests.rs:101467:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101475:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101512:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101522:    std::fs::create_dir_all(&first).unwrap();
crates/slskr/src/controller_tests.rs:101523:    std::fs::create_dir_all(&second).unwrap();
crates/slskr/src/controller_tests.rs:101536:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101575:    std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/controller_tests.rs:101596:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101620:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101633:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101654:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101655:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:101669:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101670:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:101709:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:101729:    std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:101745:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102027:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102028:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102535:    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
crates/slskr/src/controller_tests.rs:102609:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:102648:    let _ = std::fs::remove_file(&path);
crates/slskr/src/controller_tests.rs:102649:    let mut file = std::fs::OpenOptions::new()
crates/slskr/src/controller_tests.rs:102666:    std::fs::remove_file(path).expect("remove cancelled transfer test file");
crates/slskr/src/controller_tests.rs:102707:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102708:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102746:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102747:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102766:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102767:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102816:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102817:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102880:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102881:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102933:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102934:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102997:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102998:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103012:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103041:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103055:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103122:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103167:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103178:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103193:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103205:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103222:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103289:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103303:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103316:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103330:    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
crates/slskr/src/controller_tests.rs:103439:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:103446:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103461:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103473:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103487:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103543:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103576:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:103585:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:104142:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:104577:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:104582:    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
crates/slskr/src/controller_tests.rs:104753:    std::fs::remove_file(picture).unwrap();
crates/slskr/src/controller_tests.rs:104946:    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
crates/slskr/src/controller_tests.rs:104948:    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
crates/slskr/src/controller_tests.rs:105043:        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:105309:        fs::create_dir_all(&downloads_target).expect("create downloads list target");
crates/slskr/src/controller_tests.rs:105310:        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
crates/slskr/src/controller_tests.rs:105345:        let _ = fs::remove_file(downloads_link);
crates/slskr/src/controller_tests.rs:105346:        let _ = fs::remove_file(incomplete_link);
crates/slskr/src/controller_tests.rs:105347:        let _ = fs::remove_dir_all(downloads_target);
crates/slskr/src/controller_tests.rs:105348:        let _ = fs::remove_dir_all(incomplete_target);
crates/slskr/src/controller_tests.rs:105350:    let _ = fs::remove_file(downloads_conflict_root);
crates/slskr/src/controller_tests.rs:105351:    let _ = fs::remove_file(incomplete_conflict_root);
crates/slskr/src/controller_tests.rs:105604:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:105646:    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
crates/slskr/src/controller_tests.rs:105898:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106169:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106249:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106583:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106623:    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
crates/slskr/src/controller_tests.rs:106624:    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
crates/slskr/src/controller_tests.rs:106890:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106978:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:107017:    fs::remove_file(downloads_root.join("Relay/Agent.txt"))
crates/slskr/src/controller_tests.rs:107146:    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
crates/slskr/src/controller_tests.rs:107185:    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
crates/slskr/src/controller_tests.rs:107186:    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");
crates/slskr/src/controller_tests.rs:107311:    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
crates/slskr/src/controller_tests.rs:107353:    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
crates/slskr/src/controller_tests.rs:107354:    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
crates/slskr/src/controller_tests.rs:107355:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:107356:    let _ = fs::remove_dir_all(downloads_root);
crates/slskr/src/controller_tests.rs:107361:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108309:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108633:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108972:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109441:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110186:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110421:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110709:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111133:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111382:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112290:    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
crates/slskr/src/controller_tests.rs:112548:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112858:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113386:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113665:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114064:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114485:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114863:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115074:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115366:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115795:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116041:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116315:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116834:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117054:    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
crates/slskr/src/controller_tests.rs:117103:        fs::create_dir_all(&root).expect("path guard root");
crates/slskr/src/controller_tests.rs:117191:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:117273:    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
crates/slskr/src/controller_tests.rs:117376:    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
crates/slskr/src/controller_tests.rs:117539:    fs::create_dir_all(&evidence_dir).expect("create CSRF security evidence directory");
crates/slskr/src/controller_tests.rs:117668:    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
crates/slskr/src/controller_tests.rs:117682:    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");
crates/slskr/src/controller_tests.rs:117730:    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
crates/slskr/src/controller_tests.rs:117777:    fs::create_dir_all(&root).expect("certificate manager root");
crates/slskr/src/controller_tests.rs:117836:    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
crates/slskr/src/controller_tests.rs:117853:    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
crates/slskr/src/controller_tests.rs:117876:        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
crates/slskr/src/controller_tests.rs:117941:    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
crates/slskr/src/controller_tests.rs:117948:    fs::remove_dir_all(&root).expect("remove certificate manager root");
crates/slskr/src/controller_tests.rs:118116:    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
crates/slskr/src/controller_tests.rs:118262:    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
crates/slskr/src/controller_tests.rs:118629:    fs::create_dir_all(&certificate_root).expect("certificate root");
crates/slskr/src/controller_tests.rs:118658:    fs::create_dir_all(&malformed_root).expect("malformed certificate root");
crates/slskr/src/controller_tests.rs:118687:    let _ = fs::remove_dir_all(&certificate_root);
crates/slskr/src/controller_tests.rs:118688:    let _ = fs::remove_dir_all(&malformed_root);
crates/slskr/src/controller_tests.rs:118693:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:118747:    fs::create_dir_all(&root).expect("content-safety root");
crates/slskr/src/controller_tests.rs:118826:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:118830:    fs::create_dir_all(&evidence_dir).expect("create content-safety evidence directory");
crates/slskr/src/controller_tests.rs:118949:    fs::create_dir_all(&evidence_dir).expect("create Soulseek safety evidence directory");
crates/slskr/src/controller_tests.rs:119073:    fs::create_dir_all(&evidence_dir).expect("create security event sink evidence directory");
crates/slskr/src/controller_tests.rs:119619:    std::fs::create_dir_all(&evidence_dir).expect("create integrity evidence directory");
crates/slskr/src/controller_tests.rs:120298:    std::fs::create_dir_all(&evidence_dir).expect("create runtime-control evidence directory");
crates/slskr/src/controller_tests.rs:120508:    std::fs::create_dir_all(&evidence_dir).expect("create route-security evidence directory");
crates/slskr/src/controller_tests.rs:120907:    let _ = fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:121205:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121384:    fs::create_dir_all(&root).expect("JWT revocation root");
crates/slskr/src/controller_tests.rs:121429:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121434:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121555:    fs::create_dir_all(&evidence_dir).expect("create security controller evidence directory");
crates/slskr/src/controller_tests.rs:121639:    fs::create_dir_all(&evidence_dir).expect("create passthrough security evidence directory");
crates/slskr/src/controller_tests.rs:121694:        fs::create_dir_all(&root).expect("authentication control state root");
crates/slskr/src/controller_tests.rs:121853:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121859:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:121907:    fs::create_dir_all(&root).expect("pin file lifecycle root");
crates/slskr/src/controller_tests.rs:121949:        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
crates/slskr/src/controller_tests.rs:121973:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:121980:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121999:    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
crates/slskr/src/controller_tests.rs:122046:        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
crates/slskr/src/controller_tests.rs:122070:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122077:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122222:    fs::create_dir_all(&root).expect("create multisource lifecycle root");
crates/slskr/src/controller_tests.rs:122498:    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
crates/slskr/src/controller_tests.rs:122507:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122752:        let _ = fs::remove_file(yaml_failure_root);
crates/slskr/src/controller_tests.rs:122924:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:123389:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:123934:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:124112:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124182:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124340:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124395:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124556:        let _ = std::fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124606:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124849:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124987:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125131:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125188:    fs::create_dir_all(&evidence_dir).expect("create SongID persistence evidence directory");
crates/slskr/src/controller_tests.rs:125294:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125332:    fs::create_dir_all(&evidence_dir).expect("create TrafficStats evidence directory");
crates/slskr/src/controller_tests.rs:125956:    fs::create_dir_all(&evidence_dir).expect("create HashDb controller evidence directory");
crates/slskr/src/controller_tests.rs:126052:            fs::remove_file(&path).expect("remove state file before runtime failure");
crates/slskr/src/controller_tests.rs:127269:    fs::create_dir_all(&evidence_dir).expect("create PodsController evidence directory");
crates/slskr/src/controller_tests.rs:128546:    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
crates/slskr/src/controller_tests.rs:128894:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:129897:    fs::create_dir_all(&evidence_dir).expect("create RoomsController evidence directory");
crates/slskr/src/controller_tests.rs:130634:    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
crates/slskr/src/controller_tests.rs:130707:            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
crates/slskr/src/controller_tests.rs:130724:                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
crates/slskr/src/controller_tests.rs:130726:                fs::remove_file(&path).expect("remove prepared PodCore feature file");
crates/slskr/src/controller_tests.rs:132792:    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
crates/slskr/src/controller_tests.rs:133211:        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
crates/slskr/src/controller_tests.rs:133253:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/controller_tests.rs:133276:    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
crates/slskr/src/controller_tests.rs:134070:    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
crates/slskr/src/controller_tests.rs:134619:    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
crates/slskr/src/controller_tests.rs:134764:    fs::create_dir_all(&item_root).expect("create residual library directory");
crates/slskr/src/controller_tests.rs:134878:    let _ = fs::remove_dir_all(&item_root);
crates/slskr/src/controller_tests.rs:135120:    fs::create_dir_all(&evidence_dir).expect("create Library evidence directory");
crates/slskr/src/controller_tests.rs:136051:    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
crates/slskr/src/controller_tests.rs:136612:        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
crates/slskr/src/controller_tests.rs:137070:    fs::create_dir_all(&evidence_dir).expect("create Integrations evidence directory");
crates/slskr/src/controller_tests.rs:137830:    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
crates/slskr/src/controller_tests.rs:138523:    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
crates/slskr/src/controller_tests.rs:138896:    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
crates/slskr/src/controller_tests.rs:139259:    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
crates/slskr/src/controller_tests.rs:139747:    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
crates/slskr/src/controller_tests.rs:140289:    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
crates/slskr/src/controller_tests.rs:140734:    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
crates/slskr/src/controller_tests.rs:141345:    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
crates/slskr/src/controller_tests.rs:141757:    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
crates/slskr/src/controller_tests.rs:142044:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
crates/slskr/src/controller_tests.rs:142563:    let _ = fs::remove_dir_all(super::effective_downloads_dir(&controller_state));
crates/slskr/src/controller_tests.rs:142564:    let _ = fs::remove_file(share_source);
crates/slskr/src/controller_tests.rs:142569:    fs::create_dir_all(&evidence_dir).expect("create relay evidence directory");
crates/slskr/src/controller_tests.rs:143316:    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
crates/slskr/src/controller_tests.rs:144001:    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
crates/slskr/src/controller_tests.rs:144116:            fs::create_dir_all(&path).expect("create nominal directory");
crates/slskr/src/controller_tests.rs:144179:            fs::create_dir_all(&path).expect("create mutation directory");
crates/slskr/src/controller_tests.rs:144213:            fs::create_dir_all(&path).expect("create concurrent directory");
crates/slskr/src/controller_tests.rs:144251:            fs::create_dir_all(&root).expect("create file storage root");
crates/slskr/src/controller_tests.rs:144303:            fs::create_dir_all(&root).expect("create concurrent file root");
crates/slskr/src/controller_tests.rs:144351:        fs::create_dir_all(&root).expect("create incomplete mutation root");
crates/slskr/src/controller_tests.rs:144430:            fs::create_dir_all(root.join("Album")).expect("create populated root");
crates/slskr/src/controller_tests.rs:144449:            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
crates/slskr/src/controller_tests.rs:144508:            fs::create_dir_all(&album).expect("create populated detail root");
crates/slskr/src/controller_tests.rs:144534:    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");

## Async task and channel lifecycle boundaries
crates/slskr/src/route_dispatch_group_7.rs:1332:                tokio::spawn(multisource::execute(
crates/slskr/src/batch.rs:410:    fn test_batch_rejects_invalid_timeout() {
crates/slskr-client/src/quic_data.rs:615:    tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:656:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_data.rs:777:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:824:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:865:        let server = tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_6.rs:2858:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/events_ws.rs:121:    let reader_task = tokio::spawn(async move {
crates/slskr/src/events_ws.rs:123:            let frame = read_client_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/events_ws.rs:131:    let mut heartbeat = time::interval(heartbeat_interval);
crates/slskr/src/events_ws.rs:286:    time::timeout(timeout, read_client_frame(reader))
crates/slskr/src/events_ws.rs:335:    write_frame_with_timeout(writer, opcode, payload, WEBSOCKET_WRITE_TIMEOUT).await
crates/slskr/src/events_ws.rs:347:    time::timeout(timeout, write_frame_inner(writer, opcode, payload))
crates/slskr/src/events_ws.rs:486:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:491:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:520:        let message = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/events_ws.rs:642:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:665:        let (event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:689:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:692:        let error = time::timeout(
crates/slskr/src/events_ws.rs:715:            write_frame_with_timeout(&mut writer, 0x82, &payload, Duration::from_millis(50))
crates/slskr/src/events_ws.rs:724:        let error = time::timeout(
crates/slskr/src/events_ws.rs:726:            read_client_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr-client/src/quic_control.rs:253:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:386:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:403:    tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:452:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:499:        let server = tokio::spawn(async move {
crates/slskr/src/http_server.rs:187:    read_http_request_with_timeout(reader, REQUEST_READ_TIMEOUT, body_size_limit).await
crates/slskr/src/http_server.rs:195:    time::timeout(timeout, read_http_request_inner(reader, body_size_limit))
crates/slskr/src/http_server.rs:454:        time::timeout(BODY_READ_TIMEOUT, reader.read_exact(&mut buf))
crates/slskr/src/http_server.rs:666:        let available = time::timeout(timeout, reader.fill_buf())
crates/slskr/src/http_server.rs:706:    write_http_response_with_timeout(
crates/slskr/src/http_server.rs:723:    time::timeout(
crates/slskr/src/http_server.rs:870:                time::timeout(RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/http_server.rs:905:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(headers.as_bytes()))
crates/slskr/src/http_server.rs:913:            time::timeout(
crates/slskr/src/http_server.rs:926:            let read = time::timeout(RESPONSE_WRITE_TIMEOUT, file.read(&mut buffer[..wanted]))
crates/slskr/src/http_server.rs:933:            time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(&buffer[..read]))
crates/slskr/src/http_server.rs:940:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.flush())
crates/slskr/src/http_server.rs:1576:        tokio::spawn(async move {
crates/slskr/src/http_server.rs:1584:        let error = read_http_request_with_timeout(
crates/slskr/src/http_server.rs:1659:        let error = write_http_response_with_timeout(
crates/slskr/src/http_server.rs:1805:        tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_3.rs:748:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:659:        .timeout(SOURCE_TIMEOUT);
crates/slskr/src/multisource.rs:699:    timeout(deadline, resolution)
crates/slskr/src/multisource.rs:905:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:911:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:961:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1246:        let download = tokio::spawn(execute(
crates/slskr/src/multisource.rs:1322:        let server = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1348:        let fetch = tokio::spawn(async move {
crates/slskr/src/vpn.rs:213:        .timeout(Duration::from_millis(options.gluetun.timeout))
crates/slskr/src/vpn.rs:340:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:385:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:425:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:460:            let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:485:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:407:    timeout(deadline, operation)
crates/slskr/src/mesh_services.rs:553:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:567:        let fetch = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:654:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:668:        let fetch = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:130:        relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/signalr_ws.rs:158:    let reader_task = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:161:                relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT)
crates/slskr/src/signalr_ws.rs:170:    let mut keepalive = tokio::time::interval(relay_ws::SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_agent.rs:44:    tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:80:    let (mut socket, _) = time::timeout(
crates/slskr/src/relay_agent.rs:89:    let challenge = time::timeout(RELAY_REQUEST_TIMEOUT, wait_for_challenge(&mut socket))
crates/slskr/src/relay_agent.rs:108:    time::timeout(
crates/slskr/src/relay_agent.rs:117:    let share_token = time::timeout(
crates/slskr/src/relay_agent.rs:151:            messages = time::timeout(
crates/slskr/src/relay_agent.rs:234:        .timeout(RELAY_REQUEST_TIMEOUT)
crates/slskr/src/relay_agent.rs:462:    time::timeout(
crates/slskr/src/route_dispatch_group_2.rs:2855:            let interests = match time::timeout(
crates/slskr-client/src/peer_cache.rs:125:        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:129:    pub async fn send_to_with_timeout(
crates/slskr-client/src/peer_cache.rs:146:        match time::timeout(timeout, active.send(message)).await {
crates/slskr-client/src/peer_cache.rs:167:        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:171:    pub async fn receive_from_with_timeout(
crates/slskr-client/src/peer_cache.rs:187:        match time::timeout(timeout, active.receive()).await {
crates/slskr/src/dht.rs:188:        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
crates/slskr/src/dht.rs:201:                match timeout(
crates/slskr/src/dht.rs:246:        timeout(self.lookup_timeout, async {
crates/slskr-client/src/manager.rs:122:        self.ensure_peer_messages_with_timeout(username, DEFAULT_MANAGER_CONNECT_TIMEOUT)
crates/slskr-client/src/manager.rs:126:    pub async fn ensure_peer_messages_with_timeout(
crates/slskr-client/src/manager.rs:136:        time::timeout(timeout, async {
crates/slskr/src/scripts.rs:15:fn format_timeout(duration: Duration) -> String {
crates/slskr/src/scripts.rs:87:    run_with_timeout(script, script_directory, target, payload, SCRIPT_TIMEOUT).await
crates/slskr/src/scripts.rs:90:async fn run_with_timeout(
crates/slskr/src/scripts.rs:108:    let output = time::timeout(timeout_duration, command.output())
crates/slskr/src/scripts.rs:113:                format_timeout(timeout_duration)
crates/slskr/src/scripts.rs:167:        tokio::spawn(async move {
crates/slskr/src/scripts.rs:243:        let error = run_with_timeout(
crates/slskr/src/route_dispatch.rs:272:    tokio::spawn(async move {
crates/slskr-client/src/listener.rs:75:        self.accept_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:79:    pub async fn accept_with_timeout(
crates/slskr-client/src/listener.rs:83:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:101:        self.accept_obfuscated_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:105:    pub async fn accept_obfuscated_with_timeout(
crates/slskr-client/src/listener.rs:109:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:123:        self.accept_shared_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:127:    pub async fn accept_shared_with_timeout(
crates/slskr-client/src/listener.rs:131:        time::timeout(timeout, async {
crates/slskr-client/src/listener.rs:148:        self.accept_shared_mesh_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
crates/slskr-client/src/listener.rs:152:    pub async fn accept_shared_mesh_with_timeout(
crates/slskr-client/src/listener.rs:156:        time::timeout(timeout, async {
crates/slskr/src/relay_ws.rs:48:    let handshake = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/relay_ws.rs:103:    let reader_task = tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:105:            let frame = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/relay_ws.rs:113:    let mut keepalive = time::interval(SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_ws.rs:395:    time::timeout(
crates/slskr/src/relay_ws.rs:514:    time::timeout(timeout, read_ws_frame(reader))
crates/slskr/src/relay_ws.rs:526:        let error = time::timeout(
crates/slskr/src/relay_ws.rs:528:            read_ws_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr/src/route_dispatch_group_1.rs:501:                let response = tokio::time::timeout(
crates/slskr/src/route_dispatch_group_1.rs:1464:                tokio::spawn(async move {
crates/slskr-client/src/distributed_tree.rs:343:        self.send_branch_info_to_parent_with_timeout(DEFAULT_DISTRIBUTED_IO_TIMEOUT)
crates/slskr-client/src/distributed_tree.rs:347:    pub async fn send_branch_info_to_parent_with_timeout(
crates/slskr-client/src/distributed_tree.rs:359:        let result = time::timeout(timeout, async {
crates/slskr-client/src/distributed_tree.rs:385:        self.forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:393:    pub async fn forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:406:        let result = time::timeout(timeout, async {
crates/slskr/src/focused_controller_tests.rs:60:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/ftp.rs:202:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:210:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:224:            let ftp = tokio::time::timeout(timeout, AsyncRustlsFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:228:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:246:            if let Ok(Ok(ftp)) = tokio::time::timeout(timeout, secure).await {
crates/slskr/src/ftp.rs:249:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:299:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:518:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:553:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:858:            tokio::time::timeout(Duration::from_millis(50), listener.accept())
crates/slskr/src/ftp.rs:863:        let attempted = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:57:    let tcp = timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(endpoint))
crates/slskr-client/src/overlay.rs:74:    let tls = timeout(TLS_HANDSHAKE_TIMEOUT, connector.connect(server_name, tcp))
crates/slskr-client/src/overlay.rs:85:    let mut client = timeout(
crates/slskr-client/src/overlay.rs:751:        self.call_with_timeout(call, SERVICE_CALL_TIMEOUT).await
crates/slskr-client/src/overlay.rs:758:        self.search_with_timeout(request, SERVICE_CALL_TIMEOUT)
crates/slskr-client/src/overlay.rs:762:    pub async fn call_with_timeout(
crates/slskr-client/src/overlay.rs:778:        match timeout(deadline, self.call_inner(call)).await {
crates/slskr-client/src/overlay.rs:832:    pub async fn search_with_timeout(
crates/slskr-client/src/overlay.rs:848:        match timeout(deadline, self.search_inner(request)).await {
crates/slskr-client/src/overlay.rs:1261:        let task = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1285:        let writer = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1289:        let decoded = timeout(
crates/slskr-client/src/overlay.rs:1524:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1605:        assert!(timeout(Duration::from_millis(10), wire.read_u8())
crates/slskr-client/src/overlay.rs:1694:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1760:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1924:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1971:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1986:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:2016:            .call_with_timeout(&call, Duration::from_millis(10))
crates/slskr-client/src/overlay.rs:2025:                .call_with_timeout(&call, Duration::from_secs(1))
crates/slskr/src/cli.rs:559:    let stream = time::timeout(
crates/slskr/src/cli.rs:595:            let stream = time::timeout(
crates/slskr/src/cli.rs:653:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:659:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:729:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:750:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:764:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:779:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:786:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:837:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:851:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:876:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:887:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:936:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1058:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1108:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1120:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1176:            let _ = time::timeout(Duration::from_millis(750), peer.receive()).await;
crates/slskr/src/cli.rs:1188:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1331:        let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1347:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1591:            let stream = time::timeout(
crates/slskr/src/cli.rs:1777:        match time::timeout(remaining, session.receive()).await {
crates/slskr/src/cli.rs:1845:        match time::timeout(remaining, session.receive()).await {
crates/slskr/src/cli.rs:1877:    match time::timeout(Duration::from_secs(2), session.receive()).await {
crates/slskr/src/cli.rs:1924:            let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:1989:        match time::timeout(remaining, distributed.receive()).await {
crates/slskr/src/cli.rs:2030:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:2044:    let echoed = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli.rs:2228:            accept_result = listener.accept_with_timeout(remaining.min(Duration::from_secs(3))) => {
crates/slskr/src/cli.rs:2385:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2443:    time::timeout(timeout, server_task)
crates/slskr/src/cli.rs:2476:        let received = time::timeout(timeout, parent_peer.receive())
crates/slskr/src/cli.rs:2508:    let received = time::timeout(timeout, second_peer.receive())
crates/slskr/src/cli.rs:2515:    if time::timeout(Duration::from_millis(25), first_peer.receive())
crates/slskr/src/cli.rs:2622:        match time::timeout(remaining, first.receive()).await {
crates/slskr/src/cli.rs:2684:    match time::timeout(Duration::from_secs(2), TcpStream::connect(address)).await {
crates/slskr/src/cli.rs:2713:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2747:    let result = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:2753:    time::timeout(timeout, server_task)
crates/slskr/src/cli.rs:2785:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2866:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:2885:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:2897:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:2932:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2984:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:3033:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:3064:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:3099:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:3178:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:3197:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:3209:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:3390:        tokio::spawn(async move { run_listener(listener, listener_duration).await });
crates/slskr/src/cli.rs:3393:        tokio::spawn(async move { run_obfuscated_listener(listener, duration).await })
crates/slskr/src/cli.rs:3396:    let watchdog_task = tokio::spawn(run_live_soak_server_watchdog(
crates/slskr/src/cli.rs:3474:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:3544:    let accept_task = tokio::spawn(async move { listener.accept_obfuscated().await });
crates/slskr/src/cli.rs:3610:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:3712:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:3713:    let stream = time::timeout(timeout, TcpStream::connect(connect_address.as_str()))
crates/slskr/src/cli.rs:3722:    let (incoming, _) = time::timeout(timeout, accept_task)
crates/slskr/src/cli.rs:3780:        match time::timeout(
crates/slskr/src/cli.rs:3822:        match time::timeout(
crates/slskr/src/cli.rs:3917:        match time::timeout(
crates/slskr/src/cli.rs:3979:        match time::timeout(
crates/slskr/src/cli.rs:4018:        match time::timeout(
crates/slskr/src/cli.rs:4062:        match time::timeout(
crates/slskr/src/cli.rs:4125:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:4141:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:4189:        match time::timeout(
crates/slskr/src/cli.rs:4308:            time::timeout(send_timeout, session.send_ping())
crates/slskr/src/cli.rs:4319:                time::timeout(
crates/slskr/src/cli.rs:4339:        match time::timeout(next_wait, session.receive()).await {
crates/slskr/src/cli.rs:4470:            tokio::spawn(async move {
crates/slskr/src/cli.rs:4471:                match time::timeout(timeout, handle_live_soak_connect_to_peer_response(response))
crates/slskr/src/cli.rs:4546:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:4566:            let peer_response = match time::timeout(remaining, peer.receive()).await {
crates/slskr/src/cli.rs:4608:        let token = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli.rs:4633:        match time::timeout(
crates/slskr/src/cli.rs:4646:                tokio::spawn(async move {
crates/slskr/src/cli.rs:4674:        match time::timeout(
crates/slskr/src/cli.rs:4684:                tokio::spawn(async move {
crates/slskr/src/cli.rs:4715:            match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli.rs:4766:            let message = time::timeout(Duration::from_secs(5), distributed.receive())
crates/slskr/src/cli.rs:4783:            let token = time::timeout(Duration::from_secs(5), transfer.receive_token())
crates/slskr/src/cli.rs:4802:        match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli.rs:4848:    match time::timeout(Duration::from_secs(5), peer.receive_user_info_request()).await {
crates/slskr/src/cli.rs:5384:        let failed = tokio::spawn(async { Err("fixture send failed".to_owned()) });
crates/slskr/src/cli.rs:5392:        let completed = tokio::spawn(async { Ok(()) });
crates/slskr-client/src/transfer.rs:156:        self.receive_file_from_with_timeout(
crates/slskr-client/src/transfer.rs:204:        let result = time::timeout(timeout, async {
crates/slskr-client/src/transfer.rs:451:        self.send_file_to_with_timeout(connection, bytes, DEFAULT_TRANSFER_IO_TIMEOUT)
crates/slskr-client/src/transfer.rs:481:        let result = time::timeout(timeout, async {
crates/slskr/src/persistence.rs:1117:            .busy_timeout(Duration::from_secs(30));
crates/slskr/src/dotnet_regex.rs:58:    pub fn is_match_with_timeout(&self, value: &str, timeout: Duration) -> Result<bool, String> {
crates/slskr/src/dotnet_regex.rs:76:        match receiver.recv_timeout(timeout) {
crates/slskr-client/src/stream.rs:35:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:42:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/search.rs:75:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:122:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:123:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/search.rs:153:    pub fn set_server_interval(&mut self, seconds: Option<u64>) {
crates/slskr-client/src/peer_connect.rs:210:    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:238:    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:266:    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:295:    time::timeout(timeout, future)
crates/slskr/src/mesh_sync.rs:316:    let result = tokio::task::spawn_blocking(move || read_file_chunk(path, offset, length)).await;
crates/slskr/src/port_forwarding.rs:105:        let task = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:121:            if timeout(Duration::from_secs(5), &mut task).await.is_err() {
crates/slskr/src/port_forwarding.rs:184:                            tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:281:        let mut send = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:298:        let mut receive = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:324:        let _ = timeout(TUNNEL_CLOSE_TIMEOUT, close_tunnel(&client, &tunnel_id)).await;
crates/slskr/src/port_forwarding.rs:455:    let reply = timeout(SERVICE_CALL_TIMEOUT, async {
crates/slskr/src/port_forwarding.rs:627:        timeout(Duration::from_secs(1), async {
crates/slskr/src/port_forwarding.rs:679:        let stalled_gateway = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:693:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:706:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:773:        let gateway = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:862:        timeout(Duration::from_secs(5), local.read_exact(&mut echoed))
crates/slskr/src/port_forwarding.rs:867:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:879:        timeout(Duration::from_secs(5), gateway)
crates/slskr/src/webhooks.rs:605:                .timeout(timeout)
crates/slskr/src/webhooks.rs:669:            tokio::spawn(async move {
crates/slskr/src/webhooks.rs:773:            .timeout(request_timeout)
crates/slskr/src/webhooks.rs:896:    tokio::time::timeout(timeout, resolution)
crates/slskr/src/webhooks.rs:1042:        let server = tokio::spawn(async move {
crates/slskr/src/webhooks.rs:1074:        let server = tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:570:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:658:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:672:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:678:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:707:            tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:840:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:870:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:886:                match timeout(QUIC_DATA_READ_TIMEOUT, connection.accept_inbound_stream()).await {
crates/slskr/src/private_gateway.rs:912:                        match timeout(QUIC_DATA_READ_TIMEOUT, receive.read_to_end()).await {
crates/slskr/src/private_gateway.rs:941:        let (line, line_bytes) = match read_quic_data_command_line_with_timeout(&mut receive).await
crates/slskr/src/private_gateway.rs:964:            let relay_line = match read_quic_data_command_line_with_timeout(&mut receive).await {
crates/slskr/src/private_gateway.rs:1002:                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
crates/slskr/src/private_gateway.rs:1010:            if timeout(DESTINATION_WRITE_TIMEOUT, send.write_all(b"OK\n"))
crates/slskr/src/private_gateway.rs:1025:            let _ = timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await;
crates/slskr/src/private_gateway.rs:1030:        let remaining = match timeout(
crates/slskr/src/private_gateway.rs:1061:                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
crates/slskr/src/private_gateway.rs:1133:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:1340:async fn read_quic_data_command_line_with_timeout(
crates/slskr/src/private_gateway.rs:1343:    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
crates/slskr/src/private_gateway.rs:1352:    timeout(DESTINATION_WRITE_TIMEOUT, async {
crates/slskr/src/private_gateway.rs:1453:        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
crates/slskr/src/private_gateway.rs:1464:        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
crates/slskr/src/private_gateway.rs:1530:                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
crates/slskr/src/private_gateway.rs:1633:        let search = timeout(Duration::from_secs(5), async {
crates/slskr/src/private_gateway.rs:1849:        let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:2067:        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
crates/slskr/src/private_gateway.rs:2095:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2124:        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))
crates/slskr/src/private_gateway.rs:2245:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2517:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2527:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2987:        let forwarder = tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:2995:        let (size, source) = tokio::time::timeout(
crates/slskr/src/config.rs:1101:        let reconnect_delay = validated_runtime_interval(
crates/slskr/src/config.rs:1110:        let ping_interval = validated_runtime_interval(
crates/slskr/src/config.rs:1302:        let peer_response_timeout = validated_runtime_interval(
crates/slskr/src/config.rs:2708:fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
crates/slskr/src/config.rs:7533:        let timeout_connect = parse_timeout(
crates/slskr/src/config.rs:7544:        let timeout_inactivity = parse_timeout(
crates/slskr/src/config.rs:7559:        let timeout_transfer = parse_timeout(
crates/slskr/src/lib.rs:7615:    fn compile_with_timeout(
crates/slskr/src/lib.rs:7633:                .is_match_with_timeout(value, timeout)
crates/slskr/src/lib.rs:7642:fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
crates/slskr/src/lib.rs:7651:    let match_timeout = controller_regex_timeout(target);
crates/slskr/src/lib.rs:7655:            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
crates/slskr/src/lib.rs:15172:        .timeout(Duration::from_secs(10))
crates/slskr/src/lib.rs:15199:    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
crates/slskr/src/lib.rs:15342:        .timeout(Duration::from_secs(20))
crates/slskr/src/lib.rs:15418:        if let Some(metadata) = tokio::time::timeout(
crates/slskr/src/lib.rs:15585:        tokio::spawn(async move {
crates/slskr/src/lib.rs:18151:    tokio::spawn(async move {
crates/slskr/src/lib.rs:18166:    let _ = time::timeout(
crates/slskr/src/lib.rs:18181:    tokio::spawn(async move {
crates/slskr/src/lib.rs:21642:                 tokio::spawn(async move {
crates/slskr/src/lib.rs:24734:            let interests = match time::timeout(
crates/slskr/src/lib.rs:25857:                tokio::spawn(async move {
crates/slskr/src/lib.rs:33212:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:35282:                tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:36995:    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/lib.rs:37362:    tokio::spawn(async move {
crates/slskr/src/lib.rs:37415:            .timeout(Duration::from_secs(100))
crates/slskr/src/lib.rs:39524:    tokio::spawn(async move {
crates/slskr/src/lib.rs:39528:        let mut interval = time::interval(Duration::from_millis(200));
crates/slskr/src/lib.rs:43899:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:43920:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44154:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44208:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44836:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45131:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45170:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45201:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45228:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:46048:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46090:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46850:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46984:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47595:    tokio::spawn(async move {
crates/slskr/src/lib.rs:47617:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47793:        .timeout(timeout)
crates/slskr/src/lib.rs:49522:    tokio::spawn(async move {
crates/slskr/src/lib.rs:51184:                tokio::spawn(async move {
crates/slskr/src/lib.rs:54264:    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
crates/slskr/src/lib.rs:54274:    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
crates/slskr/src/lib.rs:54584:    tokio::spawn(async move {
crates/slskr/src/lib.rs:54665:        tokio::spawn(async move {
crates/slskr/src/lib.rs:55692:        .timeout(std::time::Duration::from_secs(5))
crates/slskr/src/lib.rs:56429:    let reply = match time::timeout(
crates/slskr/src/lib.rs:56850:            .timeout(solid.timeout)
crates/slskr/src/lib.rs:57271:        tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:71635:    let response = time::timeout(
crates/slskr/src/lib.rs:71683:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/lib.rs:72499:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72506:        tokio::spawn(dht.run());
crates/slskr/src/lib.rs:72564:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72570:                tokio::spawn(async move {
crates/slskr/src/lib.rs:72595:            tokio::spawn(async move {
crates/slskr/src/lib.rs:72602:                    tokio::spawn(async move {
crates/slskr/src/lib.rs:72661:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72771:    tokio::spawn(async move {
crates/slskr/src/lib.rs:72797:                wishlist_scheduler.set_server_interval(server_interval);
crates/slskr/src/lib.rs:72811:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:72862:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/lib.rs:72865:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/lib.rs:72869:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:72984:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73028:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73188:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73189:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/lib.rs:73198:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73200:        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
crates/slskr/src/lib.rs:73220:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73253:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73255:        let mut interval = time::interval(Duration::from_secs(30 * 60));
crates/slskr/src/lib.rs:73341:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73384:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73413:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73415:        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
crates/slskr/src/lib.rs:73531:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73532:        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
crates/slskr/src/lib.rs:74149:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:74156:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:74306:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74438:                            tokio::spawn(async move {
crates/slskr/src/lib.rs:74503:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:74562:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:74880:            tokio::spawn(async move {
crates/slskr/src/lib.rs:75199:    let stream = time::timeout(
crates/slskr/src/lib.rs:75233:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:75290:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:75339:            received = time::timeout(
crates/slskr/src/lib.rs:75366:                    if time::timeout(
crates/slskr/src/lib.rs:75827:        let remote_token = time::timeout(
crates/slskr/src/lib.rs:75910:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:76504:    let response = time::timeout(
crates/slskr/src/lib.rs:76562:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:76601:    time::timeout(
crates/slskr/src/lib.rs:76614:    time::timeout(
crates/slskr/src/lib.rs:76850:    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
crates/slskr/src/lib.rs:76912:    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
crates/slskr/src/lib.rs:77020:    tokio::task::spawn_blocking(move || create_application_dump_file(&state_dir))
crates/slskr/src/lib.rs:77462:        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
crates/slskr/src/lib.rs:77469:        time::timeout(io_timeout, preview.connection.send_offset(0))
crates/slskr/src/lib.rs:77479:    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
crates/slskr/src/lib.rs:77488:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:77495:            time::timeout(io_timeout, writer.write_all(&chunk))
crates/slskr/src/lib.rs:77502:    time::timeout(io_timeout, writer.flush())
crates/slskr/src/lib.rs:77524:    time::timeout(io_timeout, async {
crates/slskr/src/lib.rs:79773:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/lib.rs:80025:    tokio::spawn(async move {
crates/slskr/src/lib.rs:80776:    tokio::spawn(async move {
crates/slskr/src/lib.rs:81267:    time::timeout(
crates/slskr/src/lib.rs:81516:            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
crates/slskr/src/lib.rs:81536:    let received_token = time::timeout(
crates/slskr/src/lib.rs:81546:    time::timeout(
crates/slskr/src/lib.rs:81555:    time::timeout(
crates/slskr/src/lib.rs:82048:    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
crates/slskr/src/lib.rs:82094:        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
crates/slskr/src/lib.rs:82390:        time::timeout(
crates/slskr/src/lib.rs:82398:    let offset = time::timeout(
crates/slskr/src/lib.rs:82433:        time::timeout(
crates/slskr/src/lib.rs:82849:    let token = time::timeout(
crates/slskr/src/lib.rs:82862:    time::timeout(
crates/slskr/src/lib.rs:82878:        let chunk = time::timeout(
crates/slskr/src/lib.rs:83162:    let stream = time::timeout(settings.timeout_connect, async {
crates/slskr/src/lib.rs:83364:                    Ok(stream) => time::timeout(
crates/slskr/src/lib.rs:83402:    let stream = time::timeout(
crates/slskr/src/lib.rs:83430:    let stream = time::timeout(
crates/slskr/src/lib.rs:83456:    let stream = time::timeout(
crates/slskr/src/lib.rs:83610:    time::timeout(
crates/slskr/src/lib.rs:83617:    let message = time::timeout(
crates/slskr/src/lib.rs:83638:    time::timeout(
crates/slskr/src/lib.rs:83649:    let message = time::timeout(
crates/slskr/src/lib.rs:83668:    let stream = time::timeout(
crates/slskr/src/lib.rs:83676:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:83680:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83696:    let stream = time::timeout(
crates/slskr/src/lib.rs:83704:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:83708:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83789:                let stream = time::timeout(
crates/slskr/src/lib.rs:83801:                time::timeout(
crates/slskr/src/lib.rs:83809:                let stream = time::timeout(
crates/slskr/src/lib.rs:83817:                time::timeout(
crates/slskr/src/lib.rs:83877:    let stream = time::timeout(
crates/slskr/src/lib.rs:83885:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83889:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83905:    let stream = time::timeout(
crates/slskr/src/lib.rs:83913:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83917:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83934:    let stream = time::timeout(
crates/slskr/src/lib.rs:83942:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83946:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83964:    let stream = time::timeout(
crates/slskr/src/lib.rs:83972:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83976:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84008:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/lib.rs:84305:        let _ = time::timeout(
crates/slskr/src/lib.rs:86508:        tokio::spawn(async move {
crates/slskr/src/lib.rs:86965:    let _ = tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:86996:    tokio::spawn(async move {
crates/slskr/src/lib.rs:86998:        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
crates/slskr/src/lib.rs:89284:    let snapshot = tokio::task::spawn_blocking(move || build_share_index(&config))
crates/slskr/src/controller_tests.rs:130:    let proxy = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:208:    let proxy = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:451:        let matcher = super::ControllerRegex::compile_with_timeout(expression, true, None)
crates/slskr/src/controller_tests.rs:465:        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", false, None)
crates/slskr/src/controller_tests.rs:468:        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", true, None)
crates/slskr/src/controller_tests.rs:478:    let matcher = super::ControllerRegex::compile_with_timeout(
crates/slskr/src/controller_tests.rs:526:    let peer_task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:584:    tokio::time::timeout(Duration::from_secs(2), peer_task)
crates/slskr/src/controller_tests.rs:599:    let peer_task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:611:        let response = tokio::time::timeout(Duration::from_secs(2), peer.receive())
crates/slskr/src/controller_tests.rs:1466:    let waiter = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:1475:    let wake = tokio::time::timeout(Duration::from_secs(1), waiter)
crates/slskr/src/controller_tests.rs:2688:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:2727:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:2781:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:3533:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:4567:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:4915:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5119:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5232:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5330:async fn spotify_source_requests_enforce_configured_timeout() {
crates/slskr/src/controller_tests.rs:5337:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5369:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5404:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5439:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5464:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5566:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5862:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7406:    let echo = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7481:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:7723:    let received = tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:7796:        tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:8572:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8578:            tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8771:    let server = tokio::spawn(async move { serve_one_stun_response(&socket, mapped).await });
crates/slskr/src/controller_tests.rs:8786:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8805:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:11329:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11358:        let versioned_response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11449:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11506:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11609:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:12907:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:13112:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:20000:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21269:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21345:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21433:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21500:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21607:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21704:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21777:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21811:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21878:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21983:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22100:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22157:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22263:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22318:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26582:    let peer = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26715:    let source = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26902:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));
crates/slskr/src/controller_tests.rs:26965:    let write = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27682:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27817:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:27959:    match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28268:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28271:            tokio::time::timeout(Duration::from_secs(1), super::bridge_read_frame(&mut first))
crates/slskr/src/controller_tests.rs:28309:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28326:    let reconnected = match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28358:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28599:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28632:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28662:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28693:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28827:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29001:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29031:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29054:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29070:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29094:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29126:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:31370:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:34821:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:34996:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:42919:    let open = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43002:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43162:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44248:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44314:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44444:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44517:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:45508:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50303:        writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50570:        pod_creates.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50605:        message_writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84528:    let token_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84540:    let profile_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:85135:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:96854:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:99039:        let first_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99081:        let second_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99139:        let partial_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:100722:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:103611:    let handler = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:103705:    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
crates/slskr/src/controller_tests.rs:103706:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:104843:    tokio::time::timeout(Duration::from_secs(1), async {
crates/slskr/src/controller_tests.rs:104871:    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
crates/slskr/src/controller_tests.rs:106924:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:110756:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:113936:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:116088:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:118188:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122131:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122137:                tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122349:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122414:    let download = tokio::spawn(super::multisource::execute(
crates/slskr/src/controller_tests.rs:122421:    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/controller_tests.rs:123052:    let version_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:123594:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:136105:        let task = tokio::spawn(async move { serve_json_fixture(&listener, response).await });
crates/slskr/src/controller_tests.rs:141873:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:142498:    let stream_task = tokio::spawn(async move { live_get(stream_state, &stream_path).await });

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
dashboard/src/pages/Monitoring.tsx:120:          target="_blank"
web/scripts/audit-react-webui.mjs:614:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:615:      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
web/scripts/audit-react-webui.mjs:616:      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
web/scripts/audit-react-webui.mjs:618:        window.localStorage.setItem(
web/scripts/capture-readme-screenshots.mjs:311:  window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/capture-readme-screenshots.mjs:312:  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');
dashboard/src/components/Sidebar.tsx:60:            target="_blank"
dashboard/src/components/Sidebar.tsx:69:            target="_blank"
web/e2e/helpers.ts:254:        sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:255:        localStorage.getItem('slskr-token')
web/e2e/helpers.ts:273:        localStorage: Object.keys(localStorage).map((k) => ({
web/e2e/helpers.ts:275:          value: localStorage.getItem(k)?.slice(0, 50),
web/e2e/helpers.ts:277:        sessionStorage: Object.keys(sessionStorage).map((k) => ({
web/e2e/helpers.ts:279:          value: sessionStorage.getItem(k)?.slice(0, 50),
web/e2e/helpers.ts:482:        sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:483:        localStorage.getItem('slskr-token');
web/e2e/helpers.ts:681:      sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:682:      localStorage.getItem('slskr-token') ||
web/src/lib/session.js:18:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:31:  setToken(sessionStorage, token);
web/src/components/Rooms/Rooms.jsx:23:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:41:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:97:  // Save tabs to localStorage whenever they change
web/src/components/Browse/Browse.jsx:9:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:27:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:92:  // Save tabs to localStorage whenever they change
web/src/components/System/ExperienceSettings/index.jsx:86:    const stored = JSON.parse(localStorage.getItem(storageKey) || '{}');
web/src/components/System/ExperienceSettings/index.jsx:116:    localStorage.setItem(storageKey, JSON.stringify(form));
web/src/components/System/ExperienceSettings/index.jsx:121:    localStorage.removeItem(storageKey);
web/src/lib/searches.js:72:// Blocked users management (localStorage-based)
web/src/components/Chat/Chat.jsx:20:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:38:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:164:  // Save tabs to localStorage whenever they change
web/src/components/Shared/Footer.jsx:173:              target="_blank"
web/src/components/Shared/Footer.jsx:199:              target="_blank"
web/src/components/Shared/Footer.jsx:264:                target="_blank"
web/src/components/Shared/Footer.jsx:284:                  target="_blank"
web/src/components/Shared/Footer.jsx:293:                  target="_blank"
web/src/components/Shared/Footer.jsx:305:                  target="_blank"
web/src/components/Shared/Footer.jsx:315:                target="_blank"
web/src/lib/safeOpen.js:22:    const opened = window.open(url, '_blank', 'noopener,noreferrer');
web/src/components/Search/Detail/SearchDetail.jsx:231:  // Sync hasSavedDefault across tabs/searches when localStorage changes
web/src/lib/storage.js:5:    const value = window.localStorage.getItem(key);
web/src/lib/storage.js:16:    window.localStorage.setItem(key, value);
web/src/lib/storage.js:27:    window.localStorage.removeItem(key);
web/src/lib/storage.js:39:      { length: window.localStorage.length },
web/src/lib/storage.js:40:      (_, index) => window.localStorage.key(index),
web/src/lib/storage.js:51:    const value = window.sessionStorage.getItem(key);
web/src/lib/storage.js:62:    window.sessionStorage.setItem(key, value);
web/src/lib/storage.js:82:    window.sessionStorage.removeItem(key);
web/src/lib/communityQualitySignals.js:21:    return window.localStorage;

## Suppressed CI and script failures
.github/workflows/release.yml:348:          previous_tag="$(git describe --tags --match 'release-v*' --abbrev=0 "${GITHUB_SHA}^" 2>/dev/null || true)"
.github/workflows/release-publish.yml:273:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:380:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:419:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:462:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:574:        continue-on-error: true
scripts/run-council-scan.sh:14:    "$@" >"$tmp" || true
scripts/check-csp-policy.sh:16:    | rg -v 'assert!\(!' || true
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/start-proton-listener-soak.sh:21:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:22:sudo wg-quick down "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:23:sudo ip link del "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:24:sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/start-proton-listener-soak.sh:25:sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:37:    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:38:    sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:39:    sudo rm -rf "/etc/netns/$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:40:    sudo ip link del "$host_veth" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:42:        sudo ip route del "$endpoint_ip/32" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:44:    sudo iptables -t nat -D POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:45:    sudo iptables -D FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:46:    sudo iptables -D FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:139:sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
scripts/check-public-posture.sh:24:      | rg -v -i 'do not|should not|must not|unless|avoid|remove casual|presenting the repository|not copied|not copy|not import|not say|prohibited|forbidden|current web ui as the reference implementation|based on error type' || true
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:85:      sort -u || true
scripts/check-local-identity-leaks.sh:106:  latest_tag="$(git tag --sort=-creatordate --list 'build-main-*' | head -n 1 || true)"
scripts/check-local-identity-leaks.sh:108:    latest_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
scripts/check-web-request-body-limit-differential.sh:24:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:25:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:102:      tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:107:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:35:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:136:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:148:      tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:153:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:359:    tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:363:  wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:366:    tail -120 "$log" >&2 || true
scripts/run-council-active-bughunt.sh:35:      "$pattern" "$@" || true
scripts/run-council-active-bughunt.sh:78:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/scan-bug-council-candidates.sh:26:    "$pattern" "$@" || true
scripts/scan-bug-council-candidates.sh:73:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/check-web-no-auth-passthrough-differential.sh:28:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:29:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:110:      tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:115:  tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:298:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:305:  tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:28:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:31:        wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:37:    kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:38:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:116:      tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:121:  tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:301:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:308:  tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:89:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:105:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:125:      tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:130:  tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:139:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:146:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:51:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:52:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:118:      tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:123:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:298:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/probe-natpmp-mapping.sh:33:            "$collision_private_port" tcp 0 >/dev/null 2>&1 || true
scripts/probe-natpmp-mapping.sh:37:            "$private_port" tcp 0 >/dev/null 2>&1 || true
scripts/check-web-auth-credentials-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:49:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:50:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:126:      tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:131:  tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:535:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/with-process-memory-guard.sh:70:    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:8:  docker rm -f "$container_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:22:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:35:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:41:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:48:state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:51:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:58:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:64:  docker logs "$container_name" 2>&1 || true
scripts/validate-changelog.sh:15:unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
scripts/check-web-audit.sh:28:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-audit.sh:40:    ' <<<"$report" 2>/dev/null || true
scripts/check-web-audit.sh:54:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:30:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:119:      tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:124:  tail -120 "$log" >&2 || true
scripts/check-remediation-baseline.sh:37:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:40:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
scripts/check-rust-format.sh:63:    diff -u -- "$rust_file" "$formatted_file" || true
scripts/generate-vpn-soulseek-accounts.sh:65:  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
scripts/generate-vpn-soulseek-accounts.sh:78:  set +e
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
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
scripts/run-live-interop-matrix.sh:44:  live_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }' || true)"
scripts/run-live-interop-matrix.sh:125:    tail -n 20 "$stderr_file" || true
scripts/run-live-interop-matrix.sh:142:  set +e
scripts/run-live-interop-matrix.sh:172:set +e
scripts/run-live-interop-matrix.sh:198:set +e
scripts/run-live-interop-matrix.sh:219:set +e
scripts/check-controller-auth-profiles.sh:20:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:21:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:39:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:75:  tail -80 "$work_dir/$target.log" >&2 || true
scripts/run-slskd-api-compat-smoke.sh:36:    kill "$daemon_pid" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:37:    wait "$daemon_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:65:        renew_ports_once || true
scripts/run-live-soak-proton-natpmp.sh:75:        kill "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:76:        wait "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:80:            >/dev/null 2>&1 || true
scripts/run-live-soak-proton-natpmp.sh:84:            >/dev/null 2>&1 || true
scripts/run-cross-client-validation.sh:83:  set +e
scripts/run-cross-client-validation.sh:87:  detail="$( { tail -n 40 "$stdout_file"; grep -E '^(error:|FAILED|Failed|Build FAILED|Test Run Failed|warning |thread |panicked|Unhandled exception)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:155:  set +e
scripts/run-cross-client-validation.sh:233:  set +e
scripts/run-cross-client-validation.sh:289:    health="$(curl -fsS --max-time 2 "$health_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:290:    app="$(curl -fsS --max-time 2 "$app_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:425:    set +e
scripts/run-cross-client-validation.sh:433:    detail="$( { cat "$stdout_file"; grep -E '^(error:|thread |panicked|failed|rejected)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:458:    kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:459:    wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:477:  wait_for_daemon_preflight "$scope" "$name" "$daemon_host" "$http_port" || true
scripts/run-cross-client-validation.sh:493:      kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:494:      wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:546:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55130 || true
scripts/run-cross-client-validation.sh:568:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55131 || true
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
scripts/run-certification.sh:153:        set +e
scripts/run-certification.sh:279:        set +e
scripts/run-certification.sh:332:    set +e
scripts/run-certification.sh:349:    set +e
scripts/run-certification.sh:367:    set +e
scripts/run-certification.sh:386:        server_ip="$(getent ahostsv4 vps.slsknet.org 2>/dev/null | awk 'NR == 1 { print $1 }')" || true
scripts/run-certification.sh:420:            set +e
scripts/run-certification.sh:509:    set +e
scripts/run-certification.sh:531:    set +e
scripts/run-certification.sh:553:    set +e
scripts/run-certification.sh:575:    set +e
scripts/run-certification.sh:601:    tmux kill-session -t "$listener_session" 2>/dev/null || true
scripts/run-certification.sh:636:    set +e
scripts/run-certification.sh:664:    set +e
scripts/run-certification.sh:691:    set +e
scripts/run-certification.sh:717:    set +e
scripts/run-certification.sh:744:    set +e
scripts/run-certification.sh:803:    set +e
scripts/run-certification.sh:832:    set +e
scripts/run-certification.sh:857:    set +e
scripts/run-certification.sh:874:    set +e
scripts/run-certification.sh:900:    set +e
scripts/run-certification.sh:931:    set +e
scripts/run-certification.sh:963:    set +e
scripts/run-certification.sh:1003:    set +e
scripts/run-certification.sh:1032:    set +e
scripts/run-certification.sh:1059:    set +e
scripts/run-certification.sh:1086:    set +e
scripts/run-certification.sh:1114:        set +e
scripts/run-certification.sh:1184:    set +e
scripts/run-certification.sh:1213:    set +e
scripts/run-certification.sh:1250:        set +e
scripts/run-certification.sh:1286:            set +e
scripts/run-certification.sh:1324:    set +e
scripts/run-certification.sh:1356:        set +e
scripts/run-certification.sh:1387:        set +e
scripts/run-certification.sh:1418:    set +e
scripts/run-certification.sh:1433:    set +e
scripts/run-certification.sh:1450:        set +e
scripts/run-certification.sh:1477:    set +e
scripts/run-certification.sh:1502:    set +e
scripts/run-proton-public-matrix.sh:222:    set +e
scripts/run-proton-public-matrix.sh:302:    set +e
scripts/run-proton-public-matrix.sh:328:                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
scripts/run-proton-public-matrix.sh:334:                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
scripts/run-proton-public-matrix.sh:421:    wait_for_metadata "$listener" "$metadata_probe" || true
scripts/check-slskdn-controller-parity.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:40:      kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:42:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:95:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:109:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:110:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:124:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$slskd_report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:125:  tail -80 "$log_file" >&2 || true
scripts/check-controller-options-differential.sh:153:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:154:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:164:    kill "$soulseek_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:165:    wait "$soulseek_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:172:    kill "$listener_blocker_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:173:    wait "$listener_blocker_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:180:    kill "$lidarr_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:181:    wait "$lidarr_fixture_pid" 2>/dev/null || true
scripts/check-controller-options-differential.sh:192:    git -C "$upstream_repo" worktree remove --force "$slskd_root" >/dev/null 2>&1 || true
scripts/check-controller-options-differential.sh:195:    git -C "$upstream_repo" worktree remove --force "$slskdn_root" >/dev/null 2>&1 || true
scripts/check-controller-options-differential.sh:235:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:241:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:257:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:263:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:987:    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
scripts/check-controller-options-differential.sh:1026:      | "$python_bin" -c 'import json,sys; values=json.load(sys.stdin)["shares"]["directories"]; print(values[0] if values else "")' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1032:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1038:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1298:      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["directories"]["downloads"])' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1302:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1308:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1430:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteFileManagement"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1434:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1440:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1586:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteConfiguration"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1590:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1596:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1772:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["debug"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1776:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1782:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1953:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1960:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1971:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["pendingReconnect"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:1976:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:1983:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2221:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2228:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2253:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2260:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2453:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConfigWatch"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:2457:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2463:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2555:      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["soulseek"]["description"])' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:2559:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2565:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2589:      tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:2595:  cat "$suite/$label.error" >&2 || true
scripts/check-controller-options-differential.sh:2596:  tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:2688:      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConnect"]).lower())' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:2692:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2698:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2715:      cat "$log" >&2 || true
scripts/check-controller-options-differential.sh:2746:  cat "$status" >&2 || true
scripts/check-controller-options-differential.sh:2747:  tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:2762:    cat "$status" >&2 || true
scripts/check-controller-options-differential.sh:2763:    tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:2878:        tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2885:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:2962:      cat "$fixture_status" >&2 || true
scripts/check-controller-options-differential.sh:3055:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3061:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3236:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3242:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3411:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3417:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3570:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3576:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3616:    tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3751:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3757:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:3917:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:4053:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:4192:    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:4679:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:4685:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:4878:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:4884:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5078:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5084:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5239:      cat "$log" >&2 || true
scripts/check-controller-options-differential.sh:5263:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5269:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5301:      tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:5307:  tail -120 "$daemon_log" >&2 || true
scripts/check-controller-options-differential.sh:5536:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5542:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5557:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5563:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5575:      count="$(sqlite3 "$state/data/transfers.db" 'SELECT COUNT(*) FROM Transfers;' 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:5577:      count="$($python_bin - "$state/transfer-state.json" 2>/dev/null <<'PY' || true
scripts/check-controller-options-differential.sh:5586:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5592:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5614:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5620:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5861:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5867:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5884:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:5890:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6241:        tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6248:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6378:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:6383:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:6476:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:6570:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:6583:      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
scripts/check-controller-options-differential.sh:6588:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6659:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6666:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6695:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6702:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6820:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6826:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6856:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6862:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6887:  cat "$status" >&2 || true
scripts/check-controller-options-differential.sh:6888:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:6902:    actual="$(advertisement_count "$status" 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:6905:      actual="$(advertisement_count "$status" 2>/dev/null || true)"
scripts/check-controller-options-differential.sh:6914:  cat "$status" >&2 || true
scripts/check-controller-options-differential.sh:6915:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7224:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7230:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7353:      cat "$log" >&2 || true
scripts/check-controller-options-differential.sh:7390:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7396:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7417:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7423:  cat "$status" >&2 || true
scripts/check-controller-options-differential.sh:7424:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7432:    curl --silent --max-time 3 "$base_url/api/v0/users/fixture-peer/browse" >/dev/null || true
scripts/check-controller-options-differential.sh:7681:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7808:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7814:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7836:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:7842:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:8044:      tail -120 "$conflict_log" >&2 || true
scripts/check-controller-options-differential.sh:8132:      tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:8139:  tail -120 "$log" >&2 || true
scripts/check-controller-options-differential.sh:8594:      cat "$fixture_log" >&2 || true
scripts/check-controller-options-differential.sh:9281:    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
