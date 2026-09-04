# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Protocol-controlled allocations and lengths
crates/slskr-client/src/quic_data.rs:554:    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
crates/slskr-client/src/mesh_sync.rs:432:        let mut output = Vec::with_capacity(encoded.len());
crates/slskr-client/src/mesh_sync.rs:1030:            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
crates/slskr-client/src/quic_control.rs:41:    let mut encoded = Vec::with_capacity(key_value_len + 5);
crates/slskr-client/src/overlay_control.rs:77:        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
crates/slskr-client/src/overlay_control.rs:111:        let payload = reader.read_bytes("payload")?;
crates/slskr-client/src/overlay_control.rs:357:    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
crates/slskr-client/src/overlay.rs:212:        let mut payload = vec![0_u8; length];
crates/slskr-client/src/overlay.rs:1270:        let mut payload = vec![0; 15];
crates/slskr-client/src/overlay.rs:1501:        let mut signature = vec![0_u8; 64];
crates/slskr-client/src/search.rs:562:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr-client/src/transfer.rs:208:            connection.read_chunk(remaining).await
crates/slskr-client/src/io.rs:203:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:298:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:358:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:389:    let mut obfuscated = Vec::with_capacity(encoded_len);
crates/slskr-client/src/capabilities.rs:173:        let mut features = Vec::with_capacity(feature_count);
crates/slskr-client/src/capabilities.rs:596:    String::from_utf8(reader.read_bytes(length)?.to_vec())
crates/slskr-client/src/capabilities.rs:617:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:668:    let mut output = Vec::with_capacity(values.len());
crates/slskr-client/src/file_transfer.rs:108:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:127:        let mut chunk = vec![0; length];
crates/slskr-client/src/file_transfer.rs:147:        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
crates/slskr-client/src/file_transfer.rs:168:        let mut payload = Vec::with_capacity(length);
crates/slskr-client/src/file_transfer.rs:192:        let mut encoded = Vec::with_capacity(first_block.len() + length);
crates/slskr-protocol/src/distributed.rs:114:                    payload: reader.read_bytes(reader.remaining())?.to_vec(),
crates/slskr-protocol/src/primitives.rs:107:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:133:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:134:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:142:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:159:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:192:            output: Vec::with_capacity(capacity),
crates/slskr-client/src/listener.rs:240:        let mut encoded = Vec::with_capacity(4 + candidate_length);
crates/slskr-client/src/listener.rs:268:    let mut obfuscated = Vec::with_capacity(8 + length);
crates/slskr-client/src/listener.rs:380:            let mut nested = Vec::with_capacity(nested_len);
crates/slskr-protocol/src/peer.rs:727:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-protocol/src/peer.rs:740:        let compressed = compress_zlib(&vec![b'x'; MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES + 1])
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr/src/search_fallback.rs:37:    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
crates/slskr/src/dotnet_regex.rs:309:    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
crates/slskr/src/dotnet_regex.rs:325:    let mut named_slots = Vec::with_capacity(named.len());
crates/slskr/src/dotnet_regex.rs:347:    let mut targets = vec![String::new(); maximum_slot + 1];
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
crates/slskr/src/multisource.rs:480:        let mut sources = Vec::with_capacity(request.sources.len());
crates/slskr/src/multisource.rs:522:        let mut source_busy = vec![false; sources.len()];
crates/slskr/src/multisource.rs:526:        let mut results = Vec::with_capacity(chunks.len());
crates/slskr/src/multisource.rs:760:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/webhooks.rs:1350:        let mut persisted = vec![invalid; MAX_WEBHOOKS];
crates/slskr/src/events_ws.rs:257:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:343:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:524:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/events_ws.rs:700:        let payload = vec![b'x'; 1024 * 1024];
crates/slskr/src/bloom_filter.rs:39:            bits: vec![0_u8; bit_size.div_ceil(8)],
crates/slskr/src/port_forwarding.rs:282:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/port_forwarding.rs:742:            data: vec![7; TUNNEL_CHUNK_BYTES],
crates/slskr/src/port_forwarding.rs:752:            data: vec![7; TUNNEL_CHUNK_BYTES + 1],
crates/slskr/src/relay_ws.rs:400:    let mut header = Vec::with_capacity(10);
crates/slskr/src/relay_ws.rs:480:    let mut payload = vec![0_u8; length as usize];
crates/slskr/src/quic_alpn.rs:172:    let mut output = vec![0_u8; length];
crates/slskr/src/quic_alpn.rs:185:    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
crates/slskr/src/private_gateway.rs:1103:            let mut response = vec![0_u8; 65_536];
crates/slskr/src/private_gateway.rs:1293:    let mut bytes = Vec::with_capacity(256);
crates/slskr/src/private_gateway.rs:1296:        let read = receive.read_chunk(&mut byte).await?;
crates/slskr/src/private_gateway.rs:1360:            .read_chunk(&mut buffer[..remaining])
crates/slskr/src/private_gateway.rs:1808:            let mut bytes = vec![0_u8; length];
crates/slskr/src/private_gateway.rs:2053:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/private_gateway.rs:2924:        let mut packet = vec![0_u8; 1_200];
crates/slskr/src/private_gateway.rs:3124:            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
crates/slskr/src/content_discovery.rs:236:        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
crates/slskr/src/content_discovery.rs:245:        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
crates/slskr/src/content_discovery.rs:359:        let mut normalized = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:632:        let mut valid = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:643:        let mut candidates = Vec::with_capacity(valid.len());
crates/slskr/src/content_discovery.rs:795:    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
crates/slskr/src/content_discovery.rs:887:    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:916:    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
crates/slskr/src/mesh_sync.rs:116:            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
crates/slskr/src/mesh_sync.rs:228:    let mut incoming = Vec::with_capacity(received);
crates/slskr/src/mesh_sync.rs:298:async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
crates/slskr/src/mesh_sync.rs:352:    let mut data = vec![0_u8; to_read];
crates/slskr/src/utils.rs:713:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:731:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:1063:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr/src/relay_agent.rs:573:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay_agent.rs:724:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/route_dispatch.rs:82:    let mut normalized = Vec::with_capacity(terms.len());
crates/slskr/src/route_dispatch_group_4.rs:1833:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/relay.rs:1247:        let mut quotient = Vec::with_capacity(source.len());
crates/slskr/src/route_dispatch_group_2.rs:1804:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/cli.rs:1120:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1347:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:2897:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:3209:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:3660:        .read_chunk(5)
crates/slskr/src/http_server.rs:453:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:557:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/http_server.rs:922:        let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/http_server.rs:1078:        let body = vec![b'x'; 100 * 1024];
crates/slskr/src/security_controls.rs:1819:        let mut transformed = Vec::with_capacity(bucket + 4);
crates/slskr-web/src/lib.rs:17772:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/lib.rs:17773:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr/src/config.rs:9883:    let mut peers = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:6435:            let mut bytes = Vec::with_capacity(33);
crates/slskr/src/lib.rs:10212:        let mut updated = Vec::with_capacity(distinct_ids.len());
crates/slskr/src/lib.rs:14216:    let mut items = Vec::with_capacity(candidates.len());
crates/slskr/src/lib.rs:15433:        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:15435:            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
crates/slskr/src/lib.rs:15437:        "url" => vec!["URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:23730:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/lib.rs:28141:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/lib.rs:36658:    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
crates/slskr/src/lib.rs:46230:        let mut records = Vec::with_capacity(raw_records.len());
crates/slskr/src/lib.rs:48276:    let mut events = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:48645:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/lib.rs:49044:    let mut requested_files = Vec::with_capacity(files.len());
crates/slskr/src/lib.rs:54415:    let mut payload = vec![0_u8; length - 4];
crates/slskr/src/lib.rs:54506:    let mut provided_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:54507:    let mut configured_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:55532:    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
crates/slskr/src/lib.rs:55642:    let mut lines = Vec::with_capacity(parsed.headers.len());
crates/slskr/src/lib.rs:62108:            let mut results = Vec::with_capacity(work.len());
crates/slskr/src/lib.rs:63560:        let mut current = Vec::with_capacity(right.len() + 1);
crates/slskr/src/lib.rs:64339:        let mut results = Vec::with_capacity(descriptors.len());
crates/slskr/src/lib.rs:64483:        let mut results = Vec::with_capacity(ids.len());
crates/slskr/src/lib.rs:67732:                let mut peers = Vec::with_capacity(peer_records.len());
crates/slskr/src/lib.rs:68264:                let mut entries = Vec::with_capacity(requests.len());
crates/slskr/src/lib.rs:77403:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:81472:        connection.read_chunk(wanted),
crates/slskr/src/lib.rs:82040:    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
crates/slskr/src/lib.rs:82336:    let mut buffer = vec![0_u8; state.config.soulseek_connection.buffer_transfer];
crates/slskr/src/lib.rs:82795:            connection.read_chunk(next_len),
crates/slskr/src/lib.rs:82947:    let mut order = Vec::with_capacity(2);
crates/slskr/src/lib.rs:83140:            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
crates/slskr/src/lib.rs:83219:    let mut bound_address_and_port = vec![0_u8; address_len + 2];
crates/slskr/src/controller_tests.rs:814:        vec![0; 12]
crates/slskr/src/controller_tests.rs:2741:        let chunk = vec![b' '; 64 * 1024];
crates/slskr/src/controller_tests.rs:2787:                let chunk = vec![b'x'; 64 * 1024];
crates/slskr/src/controller_tests.rs:8725:    let mut attribute = Vec::with_capacity(8);
crates/slskr/src/controller_tests.rs:8731:    let mut response = Vec::with_capacity(32);
crates/slskr/src/controller_tests.rs:19099:        record.results = vec![template.clone(); super::MAX_SEARCH_RESULTS_PER_SEARCH];
crates/slskr/src/controller_tests.rs:21525:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:21901:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22006:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22092:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22257:    assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
crates/slskr/src/controller_tests.rs:24057:        record.members = vec![template.clone(); super::MAX_SHARE_GROUP_MEMBERS];
crates/slskr/src/controller_tests.rs:24219:        record.items = vec![template.clone(); super::MAX_COLLECTION_ITEMS];
crates/slskr/src/controller_tests.rs:28563:        let mut frame = Vec::with_capacity(4 + length as usize);
crates/slskr/src/controller_tests.rs:28678:            let mut actual = vec![0_u8; expected.len()];
crates/slskr/src/controller_tests.rs:103149:        vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103469:        vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103529:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103571:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103727:            let mut bytes = vec![0_u8; 65_536];
crates/slskr/src/controller_tests.rs:117834:        vec![0_u8; 64 * 1024 + 1],
crates/slskr/src/controller_tests.rs:119774:    let low = entropy.check(&vec![0_u8; EntropyControl::SAMPLE_SIZE]);

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/route_dispatch_group_7.rs:2152:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/route_dispatch_group_7.rs:2356:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/multisource.rs:656:    let mut builder = Client::builder()
crates/slskr/src/multisource.rs:657:        .redirect(Policy::none())
crates/slskr/src/multisource.rs:661:        builder = builder.resolve(host, SocketAddr::new(address.ip(), port));
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
crates/slskr/src/webhooks.rs:579:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:580:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:759:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:760:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:763:            client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/cli.rs:2499:    let forwarded = tree
crates/slskr/src/cli.rs:2503:    if forwarded != 1 {
crates/slskr/src/cli.rs:2505:            "distributed search reached {forwarded} children instead of one"
crates/slskr/src/application_state.rs:43:        "forwardedPort": runtime.vpn.forwarded_port,
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
crates/slskr/src/relay_agent.rs:210:) -> Result<reqwest::Client, String> {
crates/slskr/src/relay_agent.rs:211:    let mut builder = reqwest::Client::builder()
crates/slskr/src/relay_agent.rs:212:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/relay_agent.rs:539:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:605:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:705:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:749:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:782:    client: &reqwest::Client,
crates/slskr/src/private_gateway.rs:271:    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
crates/slskr/src/private_gateway.rs:2957:        .expect("DHT response should be forwarded")
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
crates/slskr/src/lib.rs:15140:        .to_socket_addrs()
crates/slskr/src/lib.rs:15150:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15152:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:15320:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15322:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:36081:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/lib.rs:36288:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:37391:        let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:37392:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:42093:                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
crates/slskr/src/lib.rs:43876:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:43878:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:43897:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:43899:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44131:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44133:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44185:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44187:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44813:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44815:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45087:        .to_socket_addrs()
crates/slskr/src/lib.rs:45108:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45110:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45147:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45149:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45178:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45180:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45205:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45207:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46025:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46027:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46030:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46067:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46069:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46072:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46827:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46829:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46832:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46961:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46963:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46966:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47594:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47596:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47599:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47770:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:47772:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48126:                .to_socket_addrs()
crates/slskr/src/lib.rs:48143:        .to_socket_addrs()
crates/slskr/src/lib.rs:48180:        .to_socket_addrs()
crates/slskr/src/lib.rs:48502:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/lib.rs:48507:fn forwarded_client_ip(
crates/slskr/src/lib.rs:48512:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/lib.rs:48513:        forwarded_header_client_ips(value)?
crates/slskr/src/lib.rs:48515:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/lib.rs:48516:        x_forwarded_for_client_ips(value)?
crates/slskr/src/lib.rs:48519:    forwarded_ips
crates/slskr/src/lib.rs:48531:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:48534:        .map(parse_forwarded_ip_token)
crates/slskr/src/lib.rs:48539:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:48542:        .map(parse_forwarded_element_ip)
crates/slskr/src/lib.rs:48547:fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:48548:    let mut forwarded_ip = None;
crates/slskr/src/lib.rs:48554:        if forwarded_ip.is_some() {
crates/slskr/src/lib.rs:48557:        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
crates/slskr/src/lib.rs:48559:    forwarded_ip
crates/slskr/src/lib.rs:48562:fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:55669:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:55671:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:55673:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:56827:        let client = match reqwest::Client::builder()
crates/slskr/src/lib.rs:56829:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:71607:        reqwest::Client::new().post(endpoint).json(&payload).send(),
crates/slskr/src/lib.rs:73018:                    "primary" => status.forwarded_port,
crates/slskr/src/lib.rs:73055:/// VPN's forwarded port. The local listener remains bound to the configured
crates/slskr/src/lib.rs:73062:            .forwarded_port
crates/slskr/src/lib.rs:84222:            reqwest::Client::new().post(endpoint).json(&payload).send(),
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
crates/slskr/src/controller_tests.rs:93482:            forwarded_port: Some(44_499),
crates/slskr/src/controller_tests.rs:93507:                && application["vpn"]["forwardedPort"] == 44_499
crates/slskr/src/controller_tests.rs:99003:        let client = reqwest::Client::new();

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
crates/slskr/src/realm_subject_index.rs:347:            fs::create_dir_all(parent)
crates/slskr/src/realm_subject_index.rs:353:        fs::rename(&temporary, path)
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
crates/slskr/src/content_discovery.rs:945:    let mut options = fs::OpenOptions::new();
crates/slskr/src/content_discovery.rs:1285:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1309:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/scripts.rs:77:    tokio::fs::create_dir_all(script_directory)
crates/slskr/src/scripts.rs:209:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:266:        tokio::fs::remove_dir_all(directory).await.unwrap();
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
crates/slskr/src/relay.rs:1214:    fs::rename(&temporary_path, &manifest_path)
crates/slskr/src/relay.rs:1417:            tokio::fs::remove_file(path)
crates/slskr/src/relay.rs:1428:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1459:        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
crates/slskr/src/relay.rs:1469:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1491:        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
crates/slskr/src/http_server.rs:1727:        std::fs::remove_file(path).unwrap();
crates/slskr/src/http_server.rs:1769:        std::fs::remove_file(path).unwrap();
crates/slskr/src/relay_agent.rs:563:    fs::create_dir_all(&relay_directory)
crates/slskr/src/relay_agent.rs:597:    let _ = fs::remove_file(&database_path).await;
crates/slskr/src/relay_agent.rs:840:            let _ = fs::remove_file(&temporary).await;
crates/slskr/src/relay_agent.rs:853:    fs::rename(&temporary, &destination)
crates/slskr/src/relay_agent.rs:881:            let _ = std::fs::remove_file(&self.path);
crates/slskr/src/mesh_security.rs:1044:                fs::create_dir_all(&mesh_directory)
crates/slskr/src/mesh_security.rs:1205:        if let Err(error) = fs::rename(&temporary, &self.storage_path) {
crates/slskr/src/mesh_security.rs:1206:            let _ = fs::remove_file(&temporary);
crates/slskr/src/persistence.rs:21:    let file = OpenOptions::new()
crates/slskr/src/persistence.rs:34:    file.set_permissions(std::fs::Permissions::from_mode(0o600))
crates/slskr/src/persistence.rs:5627:        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();
crates/slskr/src/persistence.rs:5643:        std::fs::remove_dir_all(root).unwrap();
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
crates/slskr/src/focused_controller_tests.rs:944:    fs::create_dir_all(managed_file.parent().expect("managed file parent"))
crates/slskr/src/focused_controller_tests.rs:1201:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/focused_controller_tests.rs:1205:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/focused_controller_tests.rs:1336:    fs::create_dir_all(root.join("legacy")).expect("create legacy profile root");
crates/slskr/src/focused_controller_tests.rs:1337:    fs::create_dir_all(root.join("native")).expect("create native profile root");
crates/slskr/src/focused_controller_tests.rs:1376:    let _ = fs::remove_dir_all(root);
crates/slskr/src/storage.rs:106:    OpenOptions::new()
crates/slskr/src/route_dispatch_group_2.rs:2137:                    let _ = fs::remove_file(path);
crates/slskr/src/route_dispatch_group_2.rs:2185:                    let _ = fs::remove_file(path);
crates/slskr/src/private_gateway.rs:2619:    fs::create_dir_all(state_dir)
crates/slskr/src/private_gateway.rs:2645:        return match fs::remove_file(certificate_path) {
crates/slskr/src/private_gateway.rs:2674:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2717:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2729:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2734:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2737:    if let Err(error) = fs::remove_file(&temporary) {
crates/slskr/src/private_gateway.rs:2755:        fs::create_dir_all(&path).unwrap();
crates/slskr/src/private_gateway.rs:3052:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3078:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3107:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3116:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3130:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3145:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3156:        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
crates/slskr/src/private_gateway.rs:3161:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3177:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3191:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3210:        fs::remove_dir_all(root).unwrap();
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
crates/slskr/src/lib.rs:6419:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6445:                file.set_permissions(fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:6453:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6467:            fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:12290:        let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:13971:            types: canonicalize(
crates/slskr/src/lib.rs:13984:            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
crates/slskr/src/lib.rs:13985:            statuses: canonicalize(
crates/slskr/src/lib.rs:15299:    let _ = fs::remove_file(&normalized_path);
crates/slskr/src/lib.rs:15737:    match (path.canonicalize(), root.canonicalize()) {
crates/slskr/src/lib.rs:16049:                match (normalized.canonicalize(), root.canonicalize()) {
crates/slskr/src/lib.rs:16089:    let writable = fs::OpenOptions::new()
crates/slskr/src/lib.rs:16095:        let _ = fs::remove_file(probe);
crates/slskr/src/lib.rs:16707:            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
crates/slskr/src/lib.rs:17385:            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
crates/slskr/src/lib.rs:18409:        fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:24011:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:24042:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:36679:        .canonicalize()
crates/slskr/src/lib.rs:36708:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/lib.rs:36731:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/lib.rs:36837:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:36884:    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:36885:    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:39480:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:42287:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:43763:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:43856:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:43864:    match fs::remove_file(path) {
crates/slskr/src/lib.rs:46923:    let directory = fs::canonicalize(directory)
crates/slskr/src/lib.rs:46931:        fs::remove_file(&path).map_err(|error| {
crates/slskr/src/lib.rs:50432:                                    let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:50438:                            let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:50454:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:69827:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:69844:            .canonicalize()
crates/slskr/src/lib.rs:69851:                .canonicalize()
crates/slskr/src/lib.rs:69856:                .canonicalize()
crates/slskr/src/lib.rs:71539:        fs::remove_file(path)
crates/slskr/src/lib.rs:71543:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:71548:    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
crates/slskr/src/lib.rs:72659:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/lib.rs:72671:        std::fs::create_dir_all(path)
crates/slskr/src/lib.rs:72718:    std::fs::create_dir_all(path).map_err(|error| {
crates/slskr/src/lib.rs:73854:    let _ = fs::remove_file(output_path);
crates/slskr/src/lib.rs:76636:        let canonical_path = local_path.canonicalize().ok()?;
crates/slskr/src/lib.rs:76640:            .filter_map(|root| root.canonicalize().ok())
crates/slskr/src/lib.rs:76656:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:76845:        let _ = fs::remove_file(&uploaded.path);
crates/slskr/src/lib.rs:76942:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:76947:        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
crates/slskr/src/lib.rs:76965:    let file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:76973:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77038:                let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77050:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77349:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:77356:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:77943:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:77951:            .canonicalize()
crates/slskr/src/lib.rs:77953:        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
crates/slskr/src/lib.rs:77973:            fs::remove_dir_all(&path)
crates/slskr/src/lib.rs:77979:            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/lib.rs:78119:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/lib.rs:78127:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:78131:        .canonicalize()
crates/slskr/src/lib.rs:78136:        .canonicalize()
crates/slskr/src/lib.rs:78228:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:78279:        .canonicalize()
crates/slskr/src/lib.rs:78282:        .canonicalize()
crates/slskr/src/lib.rs:78287:    fs::OpenOptions::new()
crates/slskr/src/lib.rs:81878:        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
crates/slskr/src/lib.rs:81890:            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
crates/slskr/src/lib.rs:82523:            fs::OpenOptions::new()
crates/slskr/src/lib.rs:82589:        fs::rename(&final_path, &incomplete_path)
crates/slskr/src/lib.rs:82617:        fs::remove_file(&completed_path)
crates/slskr/src/lib.rs:82620:    match fs::rename(&incomplete_path, &completed_path) {
crates/slskr/src/lib.rs:82628:            fs::remove_file(&incomplete_path)
crates/slskr/src/lib.rs:82740:        fs::create_dir_all(&root)
crates/slskr/src/lib.rs:82747:        fs::rename(path, destination)
crates/slskr/src/lib.rs:82750:        fs::remove_file(path)
crates/slskr/src/lib.rs:84192:        if tokio::fs::create_dir_all(&log_dir).await.is_ok() {
crates/slskr/src/lib.rs:84193:            if let Ok(mut file) = tokio::fs::OpenOptions::new()
crates/slskr/src/lib.rs:86794:            let _ = fs::remove_dir(&path);
crates/slskr/src/lib.rs:86797:            let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:86904:                let _ = fs::remove_file(entry.path());
crates/slskr/src/lib.rs:87951:                let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:89562:        match root.canonicalize() {
crates/slskr/src/lib.rs:89651:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/lib.rs:89914:        fs::remove_file(&rotated_path)
crates/slskr/src/lib.rs:89917:    fs::rename(path, &rotated_path)
crates/slskr/src/lib.rs:89942:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:90022:    fs::create_dir_all(parent)?;
crates/slskr/src/lib.rs:90047:        let mut file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:90058:            let _ = fs::remove_file(temp_path);
crates/slskr/src/lib.rs:90066:    fs::rename(source, destination)
crates/slskr/src/lib.rs:90074:    match fs::remove_file(destination) {
crates/slskr/src/lib.rs:90079:    fs::rename(source, destination)
crates/slskr/src/lib.rs:90104:    let mut options = fs::OpenOptions::new();
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
crates/slskr/src/controller_tests.rs:14361:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14598:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:16672:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19282:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19497:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19870:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20087:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20430:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20512:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20950:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21049:    std::fs::create_dir_all(parent).expect("download parent dir");
crates/slskr/src/controller_tests.rs:21059:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21078:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21079:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21086:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21105:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21106:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21115:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21133:    std::fs::create_dir_all(&dir).expect("test dir");
crates/slskr/src/controller_tests.rs:21139:    std::fs::remove_file(&shared_path).expect("remove shared file");
crates/slskr/src/controller_tests.rs:21149:    let _ = std::fs::remove_dir_all(dir);
crates/slskr/src/controller_tests.rs:21170:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:21171:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:21182:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:21183:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:21222:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21545:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21565:    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
crates/slskr/src/controller_tests.rs:21651:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21725:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21816:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21921:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22024:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22115:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22265:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:26343:    std::fs::create_dir_all(&root).expect("create stream share root");
crates/slskr/src/controller_tests.rs:26411:    std::fs::remove_dir_all(root).expect("remove stream fixture");
crates/slskr/src/controller_tests.rs:26446:    std::fs::create_dir_all(&root).expect("create preview share root");
crates/slskr/src/controller_tests.rs:26510:    std::fs::remove_dir_all(root).expect("remove preview fixture");
crates/slskr/src/controller_tests.rs:26826:    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
crates/slskr/src/controller_tests.rs:26914:    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");
crates/slskr/src/controller_tests.rs:26917:    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
crates/slskr/src/controller_tests.rs:26918:    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
crates/slskr/src/controller_tests.rs:26919:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:27090:    std::fs::create_dir_all(&child).unwrap();
crates/slskr/src/controller_tests.rs:27104:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:27252:    let _ = std::fs::remove_file(&queue.state_path);
crates/slskr/src/controller_tests.rs:27253:    let _ = std::fs::remove_file(&queue.events_path);
crates/slskr/src/controller_tests.rs:27725:    fs::create_dir_all(&root).expect("create overlay search state directory");
crates/slskr/src/controller_tests.rs:27850:    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
crates/slskr/src/controller_tests.rs:27860:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:27981:    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
crates/slskr/src/controller_tests.rs:28228:    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
crates/slskr/src/controller_tests.rs:28234:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28533:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:28768:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:28944:    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
crates/slskr/src/controller_tests.rs:29087:    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
crates/slskr/src/controller_tests.rs:29507:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:29683:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:31274:    fs::create_dir_all(&config.downloads_dir).unwrap();
crates/slskr/src/controller_tests.rs:31283:    fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:31294:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:31338:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:31801:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:31947:    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32224:    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
crates/slskr/src/controller_tests.rs:32285:    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
crates/slskr/src/controller_tests.rs:32286:    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32621:    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
crates/slskr/src/controller_tests.rs:32875:    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
crates/slskr/src/controller_tests.rs:33115:    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
crates/slskr/src/controller_tests.rs:33127:    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
crates/slskr/src/controller_tests.rs:33230:    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
crates/slskr/src/controller_tests.rs:34061:    std::fs::create_dir_all(&root).expect("create listening-party share root");
crates/slskr/src/controller_tests.rs:34152:    std::fs::remove_dir_all(root).expect("remove listening-party fixture");
crates/slskr/src/controller_tests.rs:34761:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:34945:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35076:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35388:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35540:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35743:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:36284:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:38918:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:38999:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39174:    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
crates/slskr/src/controller_tests.rs:39202:    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
crates/slskr/src/controller_tests.rs:40495:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:40506:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:41790:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:42680:    fs::create_dir_all(root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:42729:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:42856:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:42962:        let _ = fs::remove_file(path);
crates/slskr/src/controller_tests.rs:42965:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:43569:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:43868:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45465:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45569:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:46762:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:46910:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47100:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47316:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47517:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47774:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48067:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48788:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49100:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49485:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49694:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49734:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:49799:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:49805:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50138:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50325:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50699:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:50948:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51411:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52467:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52727:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52873:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53634:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:53923:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:54099:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54273:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54339:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54421:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54490:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54765:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55096:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55563:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55944:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56050:        fs::remove_file(&pods_path).expect("remove channel create state file");
crates/slskr/src/controller_tests.rs:56076:        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
crates/slskr/src/controller_tests.rs:56163:        fs::remove_file(&pods_path).expect("remove channel update state file");
crates/slskr/src/controller_tests.rs:56196:        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
crates/slskr/src/controller_tests.rs:56284:        fs::remove_file(&pods_path).expect("remove channel delete state file");
crates/slskr/src/controller_tests.rs:56310:        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
crates/slskr/src/controller_tests.rs:56388:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:56578:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56817:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56956:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57147:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57346:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57442:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57738:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58280:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58669:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59013:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59454:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59779:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60046:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60165:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60308:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61076:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61313:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61540:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61728:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61821:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61971:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62154:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62435:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62885:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63031:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63208:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:63464:    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
crates/slskr/src/controller_tests.rs:63598:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64022:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:64219:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64593:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64738:    fs::create_dir_all(&evidence_dir).expect("create discovery graph edge evidence directory");
crates/slskr/src/controller_tests.rs:65019:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65264:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65764:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66139:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66467:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67049:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67364:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:67592:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67783:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68156:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68592:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:69045:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:69870:    fs::create_dir_all(&evidence_dir).expect("create quarantine-jury evidence directory");
crates/slskr/src/controller_tests.rs:70109:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:70643:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:71248:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:71529:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72155:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72505:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72946:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73282:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73393:            fs::remove_file(&path).expect("remove message storage file");
crates/slskr/src/controller_tests.rs:73557:        fs::remove_dir(&messages_path).expect("remove blocked global message path");
crates/slskr/src/controller_tests.rs:73709:        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
crates/slskr/src/controller_tests.rs:73735:        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
crates/slskr/src/controller_tests.rs:73766:        fs::remove_dir(&messages_path).expect("remove blocked search message path");
crates/slskr/src/controller_tests.rs:73817:        fs::remove_dir(&messages_path).expect("remove blocked count message path");
crates/slskr/src/controller_tests.rs:73954:            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
crates/slskr/src/controller_tests.rs:73961:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74064:            fs::remove_file(&path).expect("remove membership storage file");
crates/slskr/src/controller_tests.rs:74107:        fs::remove_dir(&pods_path).expect("remove blocked membership delete path");
crates/slskr/src/controller_tests.rs:74196:        fs::remove_dir(&pods_path).expect("remove blocked membership projection path");
crates/slskr/src/controller_tests.rs:74215:        fs::remove_dir(&pods_path).expect("remove blocked membership stats path");
crates/slskr/src/controller_tests.rs:74268:        fs::remove_dir(&pods_path).expect("remove blocked membership moderation path");
crates/slskr/src/controller_tests.rs:74363:        fs::remove_dir(&pods_path).expect("remove blocked membership publish path");
crates/slskr/src/controller_tests.rs:74447:        fs::remove_dir(&pods_path).expect("remove blocked membership update path");
crates/slskr/src/controller_tests.rs:74530:        fs::remove_dir(&pods_path).expect("remove blocked membership cleanup path");
crates/slskr/src/controller_tests.rs:74559:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74627:                fs::remove_file(&path).expect("remove discovery feature state file");
crates/slskr/src/controller_tests.rs:74736:        fs::remove_dir(&feature_path).expect("remove blocked discovery registration path");
crates/slskr/src/controller_tests.rs:74824:        fs::remove_dir(&feature_path).expect("remove blocked discovery update path");
crates/slskr/src/controller_tests.rs:74937:        fs::remove_dir(&feature_path).expect("remove blocked discovery unregister path");
crates/slskr/src/controller_tests.rs:75069:        fs::remove_dir(&feature_path).expect("remove blocked discovery projection path");
crates/slskr/src/controller_tests.rs:75129:        fs::remove_dir(&feature_path).expect("remove blocked discovery refresh path");
crates/slskr/src/controller_tests.rs:75218:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:76038:    fs::create_dir_all(&evidence_dir).expect("create PodJoinLeave evidence directory");
crates/slskr/src/controller_tests.rs:76509:    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
crates/slskr/src/controller_tests.rs:76956:    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
crates/slskr/src/controller_tests.rs:77816:    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
crates/slskr/src/controller_tests.rs:78528:    fs::create_dir_all(&evidence_dir).expect("create MultiSource evidence directory");
crates/slskr/src/controller_tests.rs:78943:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79085:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79341:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79556:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79821:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80048:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80079:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81133:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81392:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:82209:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:82953:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83257:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83517:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:83818:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84023:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84229:                    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:84322:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84440:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84648:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:84653:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84773:    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:84960:    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:84965:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85155:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85499:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85748:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85825:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85923:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86013:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86233:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86412:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86514:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86577:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86647:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86689:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86741:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86796:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87119:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87296:    let _ = fs::remove_file(&validation_path);
crates/slskr/src/controller_tests.rs:87459:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87713:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87845:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:87950:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88131:    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
crates/slskr/src/controller_tests.rs:88350:    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
crates/slskr/src/controller_tests.rs:88510:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88602:    std::fs::create_dir_all(download_file.parent().unwrap())
crates/slskr/src/controller_tests.rs:88660:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:88808:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88894:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88997:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89116:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89168:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89691:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90063:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90132:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90179:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90229:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90283:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90387:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90444:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90505:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90550:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90606:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90663:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90780:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90841:    fs::create_dir_all(&custom_path).expect("create destination fixture");
crates/slskr/src/controller_tests.rs:90898:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:90902:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90957:    fs::create_dir_all(&root).expect("create destination edge root");
crates/slskr/src/controller_tests.rs:91191:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91198:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91438:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:91957:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92680:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92834:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93074:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:93343:        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
crates/slskr/src/controller_tests.rs:93398:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:93404:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93634:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93704:        std::fs::create_dir_all(&root).expect("create differential downloads root");
crates/slskr/src/controller_tests.rs:93735:        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
crates/slskr/src/controller_tests.rs:93786:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94253:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94464:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94567:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95051:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95288:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95449:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96107:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96644:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:97554:    fs::create_dir_all(existing.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:97783:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98274:    fs::create_dir_all(&new_root).unwrap();
crates/slskr/src/controller_tests.rs:98275:    fs::create_dir_all(&new_downloads).unwrap();
crates/slskr/src/controller_tests.rs:98276:    fs::create_dir_all(&new_incomplete).unwrap();
crates/slskr/src/controller_tests.rs:98674:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98705:        fs::create_dir_all(download_file.parent().unwrap()).expect("downloads fixture root");
crates/slskr/src/controller_tests.rs:98706:        fs::create_dir_all(incomplete_file.parent().unwrap()).expect("incomplete fixture root");
crates/slskr/src/controller_tests.rs:98839:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98944:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99162:        let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:99168:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99192:        fs::create_dir_all(&root).expect("secure writer root");
crates/slskr/src/controller_tests.rs:99256:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:99262:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99284:    fs::create_dir_all(&root).expect("DHT certificate root");
crates/slskr/src/controller_tests.rs:99317:        fs::create_dir_all(&linked_root).expect("DHT symlink root");
crates/slskr/src/controller_tests.rs:99375:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99382:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100311:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100336:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100337:    let _ = std::fs::remove_file(&outside);
crates/slskr/src/controller_tests.rs:100362:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100400:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100519:    std::fs::create_dir_all(&nested).expect("create nested dir");
crates/slskr/src/controller_tests.rs:100536:    std::fs::create_dir_all(&album).expect("create recursive directory");
crates/slskr/src/controller_tests.rs:100545:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100566:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100567:    std::fs::create_dir_all(&outside).expect("create outside");
crates/slskr/src/controller_tests.rs:100580:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100581:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:100598:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100613:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100633:    std::fs::create_dir_all(&directory).expect("create deep directory tree");
crates/slskr/src/controller_tests.rs:100643:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101340:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101347:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101361:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101367:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101426:    std::fs::create_dir_all(&artist).unwrap();
crates/slskr/src/controller_tests.rs:101428:    std::fs::create_dir_all(root.join(".hidden")).unwrap();
crates/slskr/src/controller_tests.rs:101445:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101453:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101490:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101500:    std::fs::create_dir_all(&first).unwrap();
crates/slskr/src/controller_tests.rs:101501:    std::fs::create_dir_all(&second).unwrap();
crates/slskr/src/controller_tests.rs:101514:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101553:    std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/controller_tests.rs:101574:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101598:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101611:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101632:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101633:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:101647:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101648:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:101687:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:101707:    std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:101723:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102005:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102006:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102513:    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
crates/slskr/src/controller_tests.rs:102587:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:102626:    let _ = std::fs::remove_file(&path);
crates/slskr/src/controller_tests.rs:102627:    let mut file = std::fs::OpenOptions::new()
crates/slskr/src/controller_tests.rs:102644:    std::fs::remove_file(path).expect("remove cancelled transfer test file");
crates/slskr/src/controller_tests.rs:102685:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102686:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102724:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102725:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102744:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102745:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102794:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102795:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102858:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102859:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102911:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102912:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102975:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102976:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102990:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103019:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103033:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103100:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103145:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103156:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103171:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103183:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103200:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103267:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103281:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103294:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103308:    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
crates/slskr/src/controller_tests.rs:103417:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:103424:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103439:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103451:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103465:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103521:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103554:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:103563:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:104120:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:104555:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:104560:    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
crates/slskr/src/controller_tests.rs:104731:    std::fs::remove_file(picture).unwrap();
crates/slskr/src/controller_tests.rs:104924:    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
crates/slskr/src/controller_tests.rs:104926:    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
crates/slskr/src/controller_tests.rs:105021:        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:105287:        fs::create_dir_all(&downloads_target).expect("create downloads list target");
crates/slskr/src/controller_tests.rs:105288:        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
crates/slskr/src/controller_tests.rs:105323:        let _ = fs::remove_file(downloads_link);
crates/slskr/src/controller_tests.rs:105324:        let _ = fs::remove_file(incomplete_link);
crates/slskr/src/controller_tests.rs:105325:        let _ = fs::remove_dir_all(downloads_target);
crates/slskr/src/controller_tests.rs:105326:        let _ = fs::remove_dir_all(incomplete_target);
crates/slskr/src/controller_tests.rs:105328:    let _ = fs::remove_file(downloads_conflict_root);
crates/slskr/src/controller_tests.rs:105329:    let _ = fs::remove_file(incomplete_conflict_root);
crates/slskr/src/controller_tests.rs:105582:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:105624:    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
crates/slskr/src/controller_tests.rs:105876:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106147:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106227:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106561:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106601:    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
crates/slskr/src/controller_tests.rs:106602:    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
crates/slskr/src/controller_tests.rs:106868:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106956:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:106995:    fs::remove_file(downloads_root.join("Relay/Agent.txt"))
crates/slskr/src/controller_tests.rs:107124:    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
crates/slskr/src/controller_tests.rs:107163:    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
crates/slskr/src/controller_tests.rs:107164:    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");
crates/slskr/src/controller_tests.rs:107289:    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
crates/slskr/src/controller_tests.rs:107331:    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
crates/slskr/src/controller_tests.rs:107332:    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
crates/slskr/src/controller_tests.rs:107333:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:107334:    let _ = fs::remove_dir_all(downloads_root);
crates/slskr/src/controller_tests.rs:107339:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108287:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108611:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108950:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109419:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110164:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110399:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110687:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111111:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111360:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112268:    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
crates/slskr/src/controller_tests.rs:112526:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112836:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113364:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113643:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114042:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114463:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114841:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115052:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115344:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115773:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116019:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116293:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116812:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117032:    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
crates/slskr/src/controller_tests.rs:117081:        fs::create_dir_all(&root).expect("path guard root");
crates/slskr/src/controller_tests.rs:117169:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:117251:    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
crates/slskr/src/controller_tests.rs:117354:    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
crates/slskr/src/controller_tests.rs:117517:    fs::create_dir_all(&evidence_dir).expect("create CSRF security evidence directory");
crates/slskr/src/controller_tests.rs:117646:    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
crates/slskr/src/controller_tests.rs:117660:    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");
crates/slskr/src/controller_tests.rs:117708:    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
crates/slskr/src/controller_tests.rs:117755:    fs::create_dir_all(&root).expect("certificate manager root");
crates/slskr/src/controller_tests.rs:117814:    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
crates/slskr/src/controller_tests.rs:117831:    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
crates/slskr/src/controller_tests.rs:117854:        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
crates/slskr/src/controller_tests.rs:117919:    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
crates/slskr/src/controller_tests.rs:117926:    fs::remove_dir_all(&root).expect("remove certificate manager root");
crates/slskr/src/controller_tests.rs:118094:    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
crates/slskr/src/controller_tests.rs:118240:    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
crates/slskr/src/controller_tests.rs:118607:    fs::create_dir_all(&certificate_root).expect("certificate root");
crates/slskr/src/controller_tests.rs:118636:    fs::create_dir_all(&malformed_root).expect("malformed certificate root");
crates/slskr/src/controller_tests.rs:118665:    let _ = fs::remove_dir_all(&certificate_root);
crates/slskr/src/controller_tests.rs:118666:    let _ = fs::remove_dir_all(&malformed_root);
crates/slskr/src/controller_tests.rs:118671:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:118725:    fs::create_dir_all(&root).expect("content-safety root");
crates/slskr/src/controller_tests.rs:118804:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:118808:    fs::create_dir_all(&evidence_dir).expect("create content-safety evidence directory");
crates/slskr/src/controller_tests.rs:118927:    fs::create_dir_all(&evidence_dir).expect("create Soulseek safety evidence directory");
crates/slskr/src/controller_tests.rs:119051:    fs::create_dir_all(&evidence_dir).expect("create security event sink evidence directory");
crates/slskr/src/controller_tests.rs:119597:    std::fs::create_dir_all(&evidence_dir).expect("create integrity evidence directory");
crates/slskr/src/controller_tests.rs:120276:    std::fs::create_dir_all(&evidence_dir).expect("create runtime-control evidence directory");
crates/slskr/src/controller_tests.rs:120486:    std::fs::create_dir_all(&evidence_dir).expect("create route-security evidence directory");
crates/slskr/src/controller_tests.rs:120885:    let _ = fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:121183:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121362:    fs::create_dir_all(&root).expect("JWT revocation root");
crates/slskr/src/controller_tests.rs:121407:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121412:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121533:    fs::create_dir_all(&evidence_dir).expect("create security controller evidence directory");
crates/slskr/src/controller_tests.rs:121617:    fs::create_dir_all(&evidence_dir).expect("create passthrough security evidence directory");
crates/slskr/src/controller_tests.rs:121672:        fs::create_dir_all(&root).expect("authentication control state root");
crates/slskr/src/controller_tests.rs:121831:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121837:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:121885:    fs::create_dir_all(&root).expect("pin file lifecycle root");
crates/slskr/src/controller_tests.rs:121927:        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
crates/slskr/src/controller_tests.rs:121951:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:121958:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121977:    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
crates/slskr/src/controller_tests.rs:122024:        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
crates/slskr/src/controller_tests.rs:122048:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122055:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122200:    fs::create_dir_all(&root).expect("create multisource lifecycle root");
crates/slskr/src/controller_tests.rs:122476:    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
crates/slskr/src/controller_tests.rs:122485:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122730:        let _ = fs::remove_file(yaml_failure_root);
crates/slskr/src/controller_tests.rs:122902:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:123367:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:123912:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:124090:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124160:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124318:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124373:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124534:        let _ = std::fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124584:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124827:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124965:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125109:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125166:    fs::create_dir_all(&evidence_dir).expect("create SongID persistence evidence directory");
crates/slskr/src/controller_tests.rs:125272:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125310:    fs::create_dir_all(&evidence_dir).expect("create TrafficStats evidence directory");
crates/slskr/src/controller_tests.rs:125934:    fs::create_dir_all(&evidence_dir).expect("create HashDb controller evidence directory");
crates/slskr/src/controller_tests.rs:126030:            fs::remove_file(&path).expect("remove state file before runtime failure");
crates/slskr/src/controller_tests.rs:127247:    fs::create_dir_all(&evidence_dir).expect("create PodsController evidence directory");
crates/slskr/src/controller_tests.rs:128524:    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
crates/slskr/src/controller_tests.rs:128872:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:129875:    fs::create_dir_all(&evidence_dir).expect("create RoomsController evidence directory");
crates/slskr/src/controller_tests.rs:130612:    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
crates/slskr/src/controller_tests.rs:130685:            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
crates/slskr/src/controller_tests.rs:130702:                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
crates/slskr/src/controller_tests.rs:130704:                fs::remove_file(&path).expect("remove prepared PodCore feature file");
crates/slskr/src/controller_tests.rs:132770:    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
crates/slskr/src/controller_tests.rs:133189:        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
crates/slskr/src/controller_tests.rs:133231:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/controller_tests.rs:133254:    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
crates/slskr/src/controller_tests.rs:134048:    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
crates/slskr/src/controller_tests.rs:134597:    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
crates/slskr/src/controller_tests.rs:134742:    fs::create_dir_all(&item_root).expect("create residual library directory");
crates/slskr/src/controller_tests.rs:134856:    let _ = fs::remove_dir_all(&item_root);
crates/slskr/src/controller_tests.rs:135098:    fs::create_dir_all(&evidence_dir).expect("create Library evidence directory");
crates/slskr/src/controller_tests.rs:136029:    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
crates/slskr/src/controller_tests.rs:136590:        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
crates/slskr/src/controller_tests.rs:137048:    fs::create_dir_all(&evidence_dir).expect("create Integrations evidence directory");
crates/slskr/src/controller_tests.rs:137808:    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
crates/slskr/src/controller_tests.rs:138501:    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
crates/slskr/src/controller_tests.rs:138874:    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
crates/slskr/src/controller_tests.rs:139237:    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
crates/slskr/src/controller_tests.rs:139725:    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
crates/slskr/src/controller_tests.rs:140267:    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
crates/slskr/src/controller_tests.rs:140712:    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
crates/slskr/src/controller_tests.rs:141323:    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
crates/slskr/src/controller_tests.rs:141735:    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
crates/slskr/src/controller_tests.rs:142022:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
crates/slskr/src/controller_tests.rs:142541:    let _ = fs::remove_dir_all(super::effective_downloads_dir(&controller_state));
crates/slskr/src/controller_tests.rs:142542:    let _ = fs::remove_file(share_source);
crates/slskr/src/controller_tests.rs:142547:    fs::create_dir_all(&evidence_dir).expect("create relay evidence directory");
crates/slskr/src/controller_tests.rs:143294:    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
crates/slskr/src/controller_tests.rs:143979:    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
crates/slskr/src/controller_tests.rs:144094:            fs::create_dir_all(&path).expect("create nominal directory");
crates/slskr/src/controller_tests.rs:144157:            fs::create_dir_all(&path).expect("create mutation directory");
crates/slskr/src/controller_tests.rs:144191:            fs::create_dir_all(&path).expect("create concurrent directory");
crates/slskr/src/controller_tests.rs:144229:            fs::create_dir_all(&root).expect("create file storage root");
crates/slskr/src/controller_tests.rs:144281:            fs::create_dir_all(&root).expect("create concurrent file root");
crates/slskr/src/controller_tests.rs:144329:        fs::create_dir_all(&root).expect("create incomplete mutation root");
crates/slskr/src/controller_tests.rs:144408:            fs::create_dir_all(root.join("Album")).expect("create populated root");
crates/slskr/src/controller_tests.rs:144427:            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
crates/slskr/src/controller_tests.rs:144486:            fs::create_dir_all(&album).expect("create populated detail root");
crates/slskr/src/controller_tests.rs:144512:    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");

## Async task and channel lifecycle boundaries
crates/slskr/src/route_dispatch_group_7.rs:1331:                tokio::spawn(multisource::execute(
crates/slskr/src/route_dispatch_group_6.rs:2857:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/dht.rs:188:        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
crates/slskr/src/dht.rs:201:                match timeout(
crates/slskr/src/dht.rs:246:        timeout(self.lookup_timeout, async {
crates/slskr/src/scripts.rs:89:        time::timeout(Duration::from_secs(300), command.output())
crates/slskr/src/scripts.rs:146:        tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:101:    let reader_task = tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:388:    time::timeout(
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
crates/slskr/src/route_dispatch_group_3.rs:747:                tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_2.rs:2854:            let interests = match time::timeout(
crates/slskr/src/events_ws.rs:120:    let reader_task = tokio::spawn(async move {
crates/slskr/src/events_ws.rs:130:    let mut heartbeat = time::interval(heartbeat_interval);
crates/slskr/src/events_ws.rs:322:    write_frame_with_timeout(writer, opcode, payload, WEBSOCKET_WRITE_TIMEOUT).await
crates/slskr/src/events_ws.rs:334:    time::timeout(timeout, write_frame_inner(writer, opcode, payload))
crates/slskr/src/events_ws.rs:473:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:478:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:507:        let message = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/events_ws.rs:629:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:652:        let (event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:676:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:679:        let error = time::timeout(
crates/slskr/src/events_ws.rs:702:            write_frame_with_timeout(&mut writer, 0x82, &payload, Duration::from_millis(50))
crates/slskr/src/batch.rs:401:    fn test_batch_rejects_invalid_timeout() {
crates/slskr/src/route_dispatch_group_1.rs:1443:                tokio::spawn(async move {
crates/slskr/src/route_dispatch.rs:272:    tokio::spawn(async move {
crates/slskr/src/focused_controller_tests.rs:60:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/mesh_sync.rs:316:    let result = tokio::task::spawn_blocking(move || read_file_chunk(path, offset, length)).await;
crates/slskr/src/mesh_services.rs:407:    timeout(deadline, operation)
crates/slskr/src/mesh_services.rs:553:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:567:        let fetch = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:654:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:668:        let fetch = tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:43:    tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:213:        .timeout(RELAY_REQUEST_TIMEOUT)
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
crates/slskr/src/vpn.rs:213:        .timeout(Duration::from_millis(options.gluetun.timeout))
crates/slskr/src/vpn.rs:340:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:385:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:425:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:460:            let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:485:        let server = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:157:    let reader_task = tokio::spawn(async move {
crates/slskr/src/webhooks.rs:605:                .timeout(timeout)
crates/slskr/src/webhooks.rs:669:            tokio::spawn(async move {
crates/slskr/src/webhooks.rs:773:            .timeout(request_timeout)
crates/slskr/src/webhooks.rs:896:    tokio::time::timeout(timeout, resolution)
crates/slskr/src/webhooks.rs:1042:        let server = tokio::spawn(async move {
crates/slskr/src/webhooks.rs:1074:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:615:    tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:656:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_data.rs:777:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:824:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:865:        let server = tokio::spawn(async move {
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
crates/slskr/src/multisource.rs:659:        .timeout(SOURCE_TIMEOUT);
crates/slskr/src/multisource.rs:699:    timeout(deadline, resolution)
crates/slskr/src/multisource.rs:905:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:911:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:961:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1246:        let download = tokio::spawn(execute(
crates/slskr/src/multisource.rs:1322:        let server = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1348:        let fetch = tokio::spawn(async move {
crates/slskr/src/dotnet_regex.rs:58:    pub fn is_match_with_timeout(&self, value: &str, timeout: Duration) -> Result<bool, String> {
crates/slskr/src/dotnet_regex.rs:76:        match receiver.recv_timeout(timeout) {
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
crates/slskr/src/private_gateway.rs:569:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:657:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:671:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:677:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:706:            tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:839:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:869:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:988:                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
crates/slskr/src/private_gateway.rs:1008:            let _ = timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await;
crates/slskr/src/private_gateway.rs:1102:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:1410:        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
crates/slskr/src/private_gateway.rs:1421:        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
crates/slskr/src/private_gateway.rs:1487:                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
crates/slskr/src/private_gateway.rs:1590:        let search = timeout(Duration::from_secs(5), async {
crates/slskr/src/private_gateway.rs:1806:        let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:2024:        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
crates/slskr/src/private_gateway.rs:2052:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2081:        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))
crates/slskr/src/private_gateway.rs:2202:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2474:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2484:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2944:        let forwarder = tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:2952:        let (size, source) = tokio::time::timeout(
crates/slskr-client/src/transfer.rs:156:        self.receive_file_from_with_timeout(
crates/slskr-client/src/transfer.rs:204:        let result = time::timeout(timeout, async {
crates/slskr-client/src/transfer.rs:451:        self.send_file_to_with_timeout(connection, bytes, DEFAULT_TRANSFER_IO_TIMEOUT)
crates/slskr-client/src/transfer.rs:481:        let result = time::timeout(timeout, async {
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
crates/slskr-client/src/stream.rs:35:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:42:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/peer_cache.rs:125:        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:129:    pub async fn send_to_with_timeout(
crates/slskr-client/src/peer_cache.rs:146:        match time::timeout(timeout, active.send(message)).await {
crates/slskr-client/src/peer_cache.rs:167:        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:171:    pub async fn receive_from_with_timeout(
crates/slskr-client/src/peer_cache.rs:187:        match time::timeout(timeout, active.receive()).await {
crates/slskr-client/src/quic_control.rs:253:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:386:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:403:    tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:452:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:499:        let server = tokio::spawn(async move {
crates/slskr-client/src/peer_connect.rs:210:    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:238:    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:266:    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:295:    time::timeout(timeout, future)
crates/slskr-client/src/manager.rs:122:        self.ensure_peer_messages_with_timeout(username, DEFAULT_MANAGER_CONNECT_TIMEOUT)
crates/slskr-client/src/manager.rs:126:    pub async fn ensure_peer_messages_with_timeout(
crates/slskr-client/src/manager.rs:142:        let connection = time::timeout(timeout, (self.connector)(username.to_owned()))
crates/slskr-client/src/distributed_tree.rs:343:        self.send_branch_info_to_parent_with_timeout(DEFAULT_DISTRIBUTED_IO_TIMEOUT)
crates/slskr-client/src/distributed_tree.rs:347:    pub async fn send_branch_info_to_parent_with_timeout(
crates/slskr-client/src/distributed_tree.rs:359:        let result = time::timeout(timeout, async {
crates/slskr-client/src/distributed_tree.rs:385:        self.forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:393:    pub async fn forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:406:        let result = time::timeout(timeout, async {
crates/slskr/src/persistence.rs:1117:            .busy_timeout(Duration::from_secs(30));
crates/slskr-client/src/search.rs:75:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:122:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:123:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/search.rs:153:    pub fn set_server_interval(&mut self, seconds: Option<u64>) {
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
crates/slskr/src/config.rs:1101:        let reconnect_delay = validated_runtime_interval(
crates/slskr/src/config.rs:1110:        let ping_interval = validated_runtime_interval(
crates/slskr/src/config.rs:1302:        let peer_response_timeout = validated_runtime_interval(
crates/slskr/src/config.rs:2708:fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
crates/slskr/src/config.rs:7533:        let timeout_connect = parse_timeout(
crates/slskr/src/config.rs:7544:        let timeout_inactivity = parse_timeout(
crates/slskr/src/config.rs:7559:        let timeout_transfer = parse_timeout(
crates/slskr/src/lib.rs:7594:    fn compile_with_timeout(
crates/slskr/src/lib.rs:7612:                .is_match_with_timeout(value, timeout)
crates/slskr/src/lib.rs:7621:fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
crates/slskr/src/lib.rs:7630:    let match_timeout = controller_regex_timeout(target);
crates/slskr/src/lib.rs:7634:            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
crates/slskr/src/lib.rs:15151:        .timeout(Duration::from_secs(10))
crates/slskr/src/lib.rs:15178:    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
crates/slskr/src/lib.rs:15321:        .timeout(Duration::from_secs(20))
crates/slskr/src/lib.rs:15397:        if let Some(metadata) = tokio::time::timeout(
crates/slskr/src/lib.rs:15564:        tokio::spawn(async move {
crates/slskr/src/lib.rs:18130:    tokio::spawn(async move {
crates/slskr/src/lib.rs:18145:    let _ = time::timeout(
crates/slskr/src/lib.rs:18160:    tokio::spawn(async move {
crates/slskr/src/lib.rs:21621:                 tokio::spawn(async move {
crates/slskr/src/lib.rs:24713:            let interests = match time::timeout(
crates/slskr/src/lib.rs:25836:                tokio::spawn(async move {
crates/slskr/src/lib.rs:33190:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:35260:                tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:36973:    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/lib.rs:37340:    tokio::spawn(async move {
crates/slskr/src/lib.rs:37393:            .timeout(Duration::from_secs(100))
crates/slskr/src/lib.rs:39502:    tokio::spawn(async move {
crates/slskr/src/lib.rs:39506:        let mut interval = time::interval(Duration::from_millis(200));
crates/slskr/src/lib.rs:43877:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:43898:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44132:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44186:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44814:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45109:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45148:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45179:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45206:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:46026:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46068:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46828:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46962:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47573:    tokio::spawn(async move {
crates/slskr/src/lib.rs:47595:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47771:        .timeout(timeout)
crates/slskr/src/lib.rs:49500:    tokio::spawn(async move {
crates/slskr/src/lib.rs:51162:                tokio::spawn(async move {
crates/slskr/src/lib.rs:54242:    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
crates/slskr/src/lib.rs:54252:    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
crates/slskr/src/lib.rs:54562:    tokio::spawn(async move {
crates/slskr/src/lib.rs:54643:        tokio::spawn(async move {
crates/slskr/src/lib.rs:55670:        .timeout(std::time::Duration::from_secs(5))
crates/slskr/src/lib.rs:56407:    let reply = match time::timeout(
crates/slskr/src/lib.rs:56828:            .timeout(solid.timeout)
crates/slskr/src/lib.rs:57249:        tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:71605:    let response = time::timeout(
crates/slskr/src/lib.rs:71653:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/lib.rs:72469:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72476:        tokio::spawn(dht.run());
crates/slskr/src/lib.rs:72534:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72540:                tokio::spawn(async move {
crates/slskr/src/lib.rs:72565:            tokio::spawn(async move {
crates/slskr/src/lib.rs:72572:                    tokio::spawn(async move {
crates/slskr/src/lib.rs:72631:        tokio::spawn(async move {
crates/slskr/src/lib.rs:72741:    tokio::spawn(async move {
crates/slskr/src/lib.rs:72767:                wishlist_scheduler.set_server_interval(server_interval);
crates/slskr/src/lib.rs:72781:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:72824:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/lib.rs:72827:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/lib.rs:72831:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:72946:    tokio::spawn(async move {
crates/slskr/src/lib.rs:72990:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73150:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73151:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/lib.rs:73160:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73162:        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
crates/slskr/src/lib.rs:73182:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73215:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73217:        let mut interval = time::interval(Duration::from_secs(30 * 60));
crates/slskr/src/lib.rs:73303:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73346:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73375:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73377:        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
crates/slskr/src/lib.rs:73493:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73494:        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
crates/slskr/src/lib.rs:74111:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:74118:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:74268:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74431:                            tokio::spawn(async move {
crates/slskr/src/lib.rs:74487:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:74795:            tokio::spawn(async move {
crates/slskr/src/lib.rs:75114:    let stream = time::timeout(
crates/slskr/src/lib.rs:75148:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:75205:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:75254:            received = time::timeout(
crates/slskr/src/lib.rs:75281:                    if time::timeout(
crates/slskr/src/lib.rs:75742:        let remote_token = time::timeout(
crates/slskr/src/lib.rs:75825:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:76419:    let response = time::timeout(
crates/slskr/src/lib.rs:76477:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:76516:    time::timeout(
crates/slskr/src/lib.rs:76529:    time::timeout(
crates/slskr/src/lib.rs:76765:    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
crates/slskr/src/lib.rs:76827:    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
crates/slskr/src/lib.rs:76935:    tokio::task::spawn_blocking(move || create_application_dump_file(&state_dir))
crates/slskr/src/lib.rs:77377:        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
crates/slskr/src/lib.rs:77384:        time::timeout(io_timeout, preview.connection.send_offset(0))
crates/slskr/src/lib.rs:77394:    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
crates/slskr/src/lib.rs:77403:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:77410:            time::timeout(io_timeout, writer.write_all(&chunk))
crates/slskr/src/lib.rs:77417:    time::timeout(io_timeout, writer.flush())
crates/slskr/src/lib.rs:77439:    time::timeout(io_timeout, async {
crates/slskr/src/lib.rs:79688:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/lib.rs:79940:    tokio::spawn(async move {
crates/slskr/src/lib.rs:80691:    tokio::spawn(async move {
crates/slskr/src/lib.rs:81182:    time::timeout(
crates/slskr/src/lib.rs:81431:            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
crates/slskr/src/lib.rs:81451:    let received_token = time::timeout(
crates/slskr/src/lib.rs:81461:    time::timeout(
crates/slskr/src/lib.rs:81470:    time::timeout(
crates/slskr/src/lib.rs:81963:    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
crates/slskr/src/lib.rs:82009:        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
crates/slskr/src/lib.rs:82305:        time::timeout(
crates/slskr/src/lib.rs:82313:    let offset = time::timeout(
crates/slskr/src/lib.rs:82348:        time::timeout(
crates/slskr/src/lib.rs:82764:    let token = time::timeout(
crates/slskr/src/lib.rs:82777:    time::timeout(
crates/slskr/src/lib.rs:82793:        let chunk = time::timeout(
crates/slskr/src/lib.rs:83077:    let stream = time::timeout(settings.timeout_connect, async {
crates/slskr/src/lib.rs:83279:                    Ok(stream) => time::timeout(
crates/slskr/src/lib.rs:83317:    let stream = time::timeout(
crates/slskr/src/lib.rs:83345:    let stream = time::timeout(
crates/slskr/src/lib.rs:83371:    let stream = time::timeout(
crates/slskr/src/lib.rs:83525:    time::timeout(
crates/slskr/src/lib.rs:83532:    let message = time::timeout(
crates/slskr/src/lib.rs:83553:    time::timeout(
crates/slskr/src/lib.rs:83564:    let message = time::timeout(
crates/slskr/src/lib.rs:83583:    let stream = time::timeout(
crates/slskr/src/lib.rs:83591:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:83595:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83611:    let stream = time::timeout(
crates/slskr/src/lib.rs:83619:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:83623:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83704:                let stream = time::timeout(
crates/slskr/src/lib.rs:83716:                time::timeout(
crates/slskr/src/lib.rs:83724:                let stream = time::timeout(
crates/slskr/src/lib.rs:83732:                time::timeout(
crates/slskr/src/lib.rs:83792:    let stream = time::timeout(
crates/slskr/src/lib.rs:83800:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83804:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83820:    let stream = time::timeout(
crates/slskr/src/lib.rs:83828:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83832:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83849:    let stream = time::timeout(
crates/slskr/src/lib.rs:83857:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83861:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83879:    let stream = time::timeout(
crates/slskr/src/lib.rs:83887:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:83891:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:83923:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/lib.rs:84220:        let _ = time::timeout(
crates/slskr/src/lib.rs:86423:        tokio::spawn(async move {
crates/slskr/src/lib.rs:86880:    let _ = tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:86911:    tokio::spawn(async move {
crates/slskr/src/lib.rs:86913:        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
crates/slskr/src/lib.rs:89199:    let snapshot = tokio::task::spawn_blocking(move || build_share_index(&config))
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
crates/slskr/src/controller_tests.rs:19978:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21247:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21323:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21411:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21478:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21585:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21682:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21755:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21789:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21856:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21961:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22078:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22135:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22241:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22296:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26560:    let peer = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26693:    let source = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26880:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));
crates/slskr/src/controller_tests.rs:26943:    let write = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27660:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27795:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:27937:    match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28246:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28249:            tokio::time::timeout(Duration::from_secs(1), super::bridge_read_frame(&mut first))
crates/slskr/src/controller_tests.rs:28287:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28304:    let reconnected = match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28336:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28577:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28610:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28640:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28671:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28805:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28979:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29009:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29032:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29048:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29072:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29104:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:31348:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:34799:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:34974:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:42897:    let open = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:42980:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43140:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44226:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44292:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44422:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44495:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:45486:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50281:        writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50548:        pod_creates.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50583:        message_writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84506:    let token_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84518:    let profile_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:85113:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:96832:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:99017:        let first_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99059:        let second_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99117:        let partial_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:100700:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:103589:    let handler = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:103683:    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
crates/slskr/src/controller_tests.rs:103684:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:104821:    tokio::time::timeout(Duration::from_secs(1), async {
crates/slskr/src/controller_tests.rs:104849:    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
crates/slskr/src/controller_tests.rs:106902:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:110734:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:113914:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:116066:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:118166:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122109:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122115:                tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122327:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122392:    let download = tokio::spawn(super::multisource::execute(
crates/slskr/src/controller_tests.rs:122399:    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/controller_tests.rs:123030:    let version_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:123572:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:136083:        let task = tokio::spawn(async move { serve_json_fixture(&listener, response).await });
crates/slskr/src/controller_tests.rs:141851:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:142476:    let stream_task = tokio::spawn(async move { live_get(stream_state, &stream_path).await });

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
web/scripts/audit-react-webui.mjs:614:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:615:      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
web/scripts/audit-react-webui.mjs:616:      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
web/scripts/audit-react-webui.mjs:618:        window.localStorage.setItem(
dashboard/src/components/Sidebar.tsx:60:            target="_blank"
dashboard/src/components/Sidebar.tsx:69:            target="_blank"
dashboard/src/pages/Monitoring.tsx:120:          target="_blank"
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
web/scripts/capture-readme-screenshots.mjs:311:  window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/capture-readme-screenshots.mjs:312:  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');
web/src/lib/communityQualitySignals.js:21:    return window.localStorage;
web/src/lib/storage.js:5:    const value = window.localStorage.getItem(key);
web/src/lib/storage.js:16:    window.localStorage.setItem(key, value);
web/src/lib/storage.js:27:    window.localStorage.removeItem(key);
web/src/lib/storage.js:39:      { length: window.localStorage.length },
web/src/lib/storage.js:40:      (_, index) => window.localStorage.key(index),
web/src/lib/storage.js:51:    const value = window.sessionStorage.getItem(key);
web/src/lib/storage.js:62:    window.sessionStorage.setItem(key, value);
web/src/lib/storage.js:82:    window.sessionStorage.removeItem(key);
web/src/lib/session.js:18:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:31:  setToken(sessionStorage, token);
web/src/lib/searches.js:72:// Blocked users management (localStorage-based)
web/src/lib/safeOpen.js:22:    const opened = window.open(url, '_blank', 'noopener,noreferrer');
web/src/components/Browse/Browse.jsx:9:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:27:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:92:  // Save tabs to localStorage whenever they change
web/src/components/Rooms/Rooms.jsx:23:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:41:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:97:  // Save tabs to localStorage whenever they change
web/src/components/Chat/Chat.jsx:20:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:38:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:164:  // Save tabs to localStorage whenever they change
web/src/components/System/ExperienceSettings/index.jsx:86:    const stored = JSON.parse(localStorage.getItem(storageKey) || '{}');
web/src/components/System/ExperienceSettings/index.jsx:116:    localStorage.setItem(storageKey, JSON.stringify(form));
web/src/components/System/ExperienceSettings/index.jsx:121:    localStorage.removeItem(storageKey);
web/src/components/Search/Detail/SearchDetail.jsx:228:  // Sync hasSavedDefault across tabs/searches when localStorage changes
web/src/components/Shared/Footer.jsx:173:              target="_blank"
web/src/components/Shared/Footer.jsx:199:              target="_blank"
web/src/components/Shared/Footer.jsx:264:                target="_blank"
web/src/components/Shared/Footer.jsx:284:                  target="_blank"
web/src/components/Shared/Footer.jsx:293:                  target="_blank"
web/src/components/Shared/Footer.jsx:305:                  target="_blank"
web/src/components/Shared/Footer.jsx:315:                target="_blank"

## Suppressed CI and script failures
scripts/check-public-posture.sh:24:      | rg -v -i 'do not|should not|must not|unless|avoid|remove casual|presenting the repository|not copied|not copy|not import|not say|prohibited|forbidden|current web ui as the reference implementation|based on error type' || true
scripts/start-proton-listener-soak.sh:21:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:22:sudo wg-quick down "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:23:sudo ip link del "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:24:sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/start-proton-listener-soak.sh:25:sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/run-in-proton-wg-netns.sh:37:    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:38:    sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:39:    sudo rm -rf "/etc/netns/$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:40:    sudo ip link del "$host_veth" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:42:        sudo ip route del "$endpoint_ip/32" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:44:    sudo iptables -t nat -D POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:45:    sudo iptables -D FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:46:    sudo iptables -D FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:139:sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/run-council-scan.sh:14:    "$@" >"$tmp" || true
scripts/check-csp-policy.sh:16:    | rg -v 'assert!\(!' || true
.github/workflows/release.yml:348:          previous_tag="$(git describe --tags --match 'release-v*' --abbrev=0 "${GITHUB_SHA}^" 2>/dev/null || true)"
scripts/with-process-memory-guard.sh:70:    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:8:  docker rm -f "$container_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:22:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:35:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:41:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:48:state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:51:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:58:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:64:  docker logs "$container_name" 2>&1 || true
scripts/check-web-audit.sh:28:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-audit.sh:40:    ' <<<"$report" 2>/dev/null || true
scripts/check-web-audit.sh:54:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/validate-changelog.sh:15:unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
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
.github/workflows/release-publish.yml:273:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:380:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:419:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:462:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:574:        continue-on-error: true
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
scripts/check-web-request-body-limit-differential.sh:24:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:25:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:102:      tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:107:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:51:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:52:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:118:      tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:123:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:298:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/check-web-rate-limiting-differential.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:30:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:119:      tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:124:  tail -120 "$log" >&2 || true
scripts/check-rust-format.sh:63:    diff -u -- "$rust_file" "$formatted_file" || true
scripts/check-web-cors-differential.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:35:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:136:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:148:      tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:153:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:359:    tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:363:  wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:366:    tail -120 "$log" >&2 || true
scripts/run-proton-public-matrix.sh:222:    set +e
scripts/run-proton-public-matrix.sh:302:    set +e
scripts/run-proton-public-matrix.sh:328:                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
scripts/run-proton-public-matrix.sh:334:                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
scripts/run-proton-public-matrix.sh:421:    wait_for_metadata "$listener" "$metadata_probe" || true
scripts/check-web-auth-credentials-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:49:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:50:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:126:      tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:131:  tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:535:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/run-council-active-bughunt.sh:35:      "$pattern" "$@" || true
scripts/run-council-active-bughunt.sh:78:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/check-remediation-baseline.sh:37:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:40:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
scripts/scan-bug-council-candidates.sh:26:    "$pattern" "$@" || true
scripts/scan-bug-council-candidates.sh:73:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:85:      sort -u || true
scripts/check-local-identity-leaks.sh:106:  latest_tag="$(git tag --sort=-creatordate --list 'build-main-*' | head -n 1 || true)"
scripts/check-local-identity-leaks.sh:108:    latest_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
scripts/check-controller-auth-profiles.sh:20:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:21:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:39:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-controller-auth-profiles.sh:75:  tail -80 "$work_dir/$target.log" >&2 || true
scripts/run-live-interop-matrix.sh:44:  live_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }' || true)"
scripts/run-live-interop-matrix.sh:125:    tail -n 20 "$stderr_file" || true
scripts/run-live-interop-matrix.sh:142:  set +e
scripts/run-live-interop-matrix.sh:172:set +e
scripts/run-live-interop-matrix.sh:198:set +e
scripts/run-live-interop-matrix.sh:219:set +e
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
scripts/run-live-soak-proton-natpmp.sh:65:        renew_ports_once || true
scripts/run-live-soak-proton-natpmp.sh:75:        kill "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:76:        wait "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:80:            >/dev/null 2>&1 || true
scripts/run-live-soak-proton-natpmp.sh:84:            >/dev/null 2>&1 || true
scripts/check-slskdn-controller-parity.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:40:      kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:42:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-slskdn-controller-parity.sh:95:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:109:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:110:  tail -80 "$log_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:124:  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$slskd_report_file" >&2 || true
scripts/check-slskdn-controller-parity.sh:125:  tail -80 "$log_file" >&2 || true
scripts/probe-natpmp-mapping.sh:33:            "$collision_private_port" tcp 0 >/dev/null 2>&1 || true
scripts/probe-natpmp-mapping.sh:37:            "$private_port" tcp 0 >/dev/null 2>&1 || true
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
scripts/generate-vpn-soulseek-accounts.sh:65:  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
scripts/generate-vpn-soulseek-accounts.sh:78:  set +e
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
scripts/run-cross-client-validation.sh:91:  set +e
scripts/run-cross-client-validation.sh:95:  detail="$( { tail -n 40 "$stdout_file"; grep -E '^(error:|FAILED|Failed|Build FAILED|Test Run Failed|warning |thread |panicked|Unhandled exception)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:163:  set +e
scripts/run-cross-client-validation.sh:241:  set +e
scripts/run-cross-client-validation.sh:297:    health="$(curl -fsS --max-time 2 "$health_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:298:    app="$(curl -fsS --max-time 2 "$app_url" 2>/dev/null | sanitize_detail || true)"
scripts/run-cross-client-validation.sh:433:    set +e
scripts/run-cross-client-validation.sh:441:    detail="$( { cat "$stdout_file"; grep -E '^(error:|thread |panicked|failed|rejected)' "$stderr_file" || true; } | sanitize_detail )"
scripts/run-cross-client-validation.sh:466:    kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:467:    wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:485:  wait_for_daemon_preflight "$scope" "$name" "$daemon_host" "$http_port" || true
scripts/run-cross-client-validation.sh:501:      kill "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:502:      wait "$pid" 2>/dev/null || true
scripts/run-cross-client-validation.sh:554:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55130 || true
scripts/run-cross-client-validation.sh:576:    wait_for_daemon_preflight slskr-to-slskr slskr "$slskr_host" 55131 || true
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
scripts/run-slskd-api-compat-smoke.sh:36:    kill "$daemon_pid" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:37:    wait "$daemon_pid" 2>/dev/null || true
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
