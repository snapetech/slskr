# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Protocol-controlled allocations and lengths
crates/slskr-protocol/src/peer.rs:727:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-protocol/src/peer.rs:740:        let compressed = compress_zlib(&vec![b'x'; MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES + 1])
crates/slskr-client/src/quic_data.rs:554:    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr-protocol/src/distributed.rs:114:                    payload: reader.read_bytes(reader.remaining())?.to_vec(),
crates/slskr-client/src/mesh_sync.rs:432:        let mut output = Vec::with_capacity(encoded.len());
crates/slskr-client/src/mesh_sync.rs:1030:            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
crates/slskr-client/src/quic_control.rs:41:    let mut encoded = Vec::with_capacity(key_value_len + 5);
crates/slskr-client/src/overlay_control.rs:77:        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
crates/slskr-client/src/overlay_control.rs:111:        let payload = reader.read_bytes("payload")?;
crates/slskr-client/src/overlay_control.rs:357:    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
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
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr-protocol/src/primitives.rs:107:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:133:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:134:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:142:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:159:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:192:            output: Vec::with_capacity(capacity),
crates/slskr-client/src/overlay.rs:212:        let mut payload = vec![0_u8; length];
crates/slskr-client/src/overlay.rs:1270:        let mut payload = vec![0; 15];
crates/slskr-client/src/overlay.rs:1501:        let mut signature = vec![0_u8; 64];
crates/slskr-client/src/search.rs:562:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr/src/bloom_filter.rs:39:            bits: vec![0_u8; bit_size.div_ceil(8)],
crates/slskr-client/src/listener.rs:240:        let mut encoded = Vec::with_capacity(4 + candidate_length);
crates/slskr-client/src/listener.rs:268:    let mut obfuscated = Vec::with_capacity(8 + length);
crates/slskr-client/src/listener.rs:380:            let mut nested = Vec::with_capacity(nested_len);
crates/slskr-client/src/capabilities.rs:173:        let mut features = Vec::with_capacity(feature_count);
crates/slskr-client/src/capabilities.rs:596:    String::from_utf8(reader.read_bytes(length)?.to_vec())
crates/slskr-client/src/capabilities.rs:617:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:668:    let mut output = Vec::with_capacity(values.len());
crates/slskr/src/content_discovery.rs:237:        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
crates/slskr/src/content_discovery.rs:246:        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
crates/slskr/src/content_discovery.rs:360:        let mut normalized = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:652:        let mut valid = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:663:        let mut candidates = Vec::with_capacity(valid.len());
crates/slskr/src/content_discovery.rs:827:    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
crates/slskr/src/content_discovery.rs:919:    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:948:    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
crates/slskr-client/src/transfer.rs:208:            connection.read_chunk(remaining).await
crates/slskr-client/src/io.rs:203:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:298:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:358:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:389:    let mut obfuscated = Vec::with_capacity(encoded_len);
crates/slskr-client/src/file_transfer.rs:108:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:127:        let mut chunk = vec![0; length];
crates/slskr-client/src/file_transfer.rs:147:        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
crates/slskr-client/src/file_transfer.rs:168:        let mut payload = Vec::with_capacity(length);
crates/slskr-client/src/file_transfer.rs:192:        let mut encoded = Vec::with_capacity(first_block.len() + length);
crates/slskr/src/relay_ws.rs:413:    let mut header = Vec::with_capacity(10);
crates/slskr/src/relay_ws.rs:493:    let mut payload = vec![0_u8; length as usize];
crates/slskr/src/dotnet_regex.rs:309:    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
crates/slskr/src/dotnet_regex.rs:325:    let mut named_slots = Vec::with_capacity(named.len());
crates/slskr/src/dotnet_regex.rs:347:    let mut targets = vec![String::new(); maximum_slot + 1];
crates/slskr/src/quic_alpn.rs:172:    let mut output = vec![0_u8; length];
crates/slskr/src/quic_alpn.rs:185:    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
crates/slskr/src/utils.rs:713:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:731:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:1063:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr/src/route_dispatch_group_2.rs:1825:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/search_fallback.rs:37:    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
crates/slskr-web/src/lib.rs:17789:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/lib.rs:17790:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr/src/events_ws.rs:258:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:356:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:537:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/events_ws.rs:713:        let payload = vec![b'x'; 1024 * 1024];
crates/slskr/src/route_dispatch_group_4.rs:1834:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/multisource.rs:480:        let mut sources = Vec::with_capacity(request.sources.len());
crates/slskr/src/multisource.rs:522:        let mut source_busy = vec![false; sources.len()];
crates/slskr/src/multisource.rs:526:        let mut results = Vec::with_capacity(chunks.len());
crates/slskr/src/multisource.rs:760:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/webhooks.rs:1350:        let mut persisted = vec![invalid; MAX_WEBHOOKS];
crates/slskr/src/mesh_sync.rs:116:            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
crates/slskr/src/mesh_sync.rs:235:    let mut incoming = Vec::with_capacity(received);
crates/slskr/src/mesh_sync.rs:336:async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
crates/slskr/src/mesh_sync.rs:390:    let mut data = vec![0_u8; to_read];
crates/slskr/src/relay_agent.rs:738:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay_agent.rs:898:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay.rs:1297:        let mut quotient = Vec::with_capacity(source.len());
crates/slskr/src/port_forwarding.rs:293:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/port_forwarding.rs:784:            data: vec![7; TUNNEL_CHUNK_BYTES],
crates/slskr/src/port_forwarding.rs:794:            data: vec![7; TUNNEL_CHUNK_BYTES + 1],
crates/slskr/src/security_controls.rs:1819:        let mut transformed = Vec::with_capacity(bucket + 4);
crates/slskr/src/http_server.rs:453:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:557:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/http_server.rs:922:        let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/http_server.rs:1078:        let body = vec![b'x'; 100 * 1024];
crates/slskr/src/private_gateway.rs:1161:            let mut response = vec![0_u8; 65_536];
crates/slskr/src/private_gateway.rs:1351:    let mut bytes = Vec::with_capacity(256);
crates/slskr/src/private_gateway.rs:1354:        let read = receive.read_chunk(&mut byte).await?;
crates/slskr/src/private_gateway.rs:1430:            .read_chunk(&mut buffer[..remaining])
crates/slskr/src/private_gateway.rs:1878:            let mut bytes = vec![0_u8; length];
crates/slskr/src/private_gateway.rs:2126:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/private_gateway.rs:2997:        let mut packet = vec![0_u8; 1_200];
crates/slskr/src/private_gateway.rs:3197:            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
crates/slskr/src/route_dispatch.rs:82:    let mut normalized = Vec::with_capacity(terms.len());
crates/slskr/src/cli.rs:1120:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1347:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:2897:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:3209:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:3660:        .read_chunk(5)
crates/slskr/src/config.rs:9900:    let mut peers = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:6517:            let mut bytes = Vec::with_capacity(33);
crates/slskr/src/lib.rs:10314:        let mut updated = Vec::with_capacity(distinct_ids.len());
crates/slskr/src/lib.rs:14375:    let mut items = Vec::with_capacity(candidates.len());
crates/slskr/src/lib.rs:15597:        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:15599:            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
crates/slskr/src/lib.rs:15601:        "url" => vec!["URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:24162:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/lib.rs:28584:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/lib.rs:37188:    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
crates/slskr/src/lib.rs:46770:        let mut records = Vec::with_capacity(raw_records.len());
crates/slskr/src/lib.rs:48828:    let mut events = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:49197:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/lib.rs:49596:    let mut requested_files = Vec::with_capacity(files.len());
crates/slskr/src/lib.rs:55073:    let mut payload = vec![0_u8; length - 4];
crates/slskr/src/lib.rs:55177:    let mut provided_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:55178:    let mut configured_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:56276:    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
crates/slskr/src/lib.rs:56386:    let mut lines = Vec::with_capacity(parsed.headers.len());
crates/slskr/src/lib.rs:62869:            let mut results = Vec::with_capacity(work.len());
crates/slskr/src/lib.rs:64321:        let mut current = Vec::with_capacity(right.len() + 1);
crates/slskr/src/lib.rs:65100:        let mut results = Vec::with_capacity(descriptors.len());
crates/slskr/src/lib.rs:65244:        let mut results = Vec::with_capacity(ids.len());
crates/slskr/src/lib.rs:68496:                let mut peers = Vec::with_capacity(peer_records.len());
crates/slskr/src/lib.rs:69028:                let mut entries = Vec::with_capacity(requests.len());
crates/slskr/src/lib.rs:78322:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:82422:        connection.read_chunk(wanted),
crates/slskr/src/lib.rs:83052:    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
crates/slskr/src/lib.rs:83348:    let mut buffer = vec![0_u8; state.config.soulseek_connection.buffer_transfer];
crates/slskr/src/lib.rs:83807:            connection.read_chunk(next_len),
crates/slskr/src/lib.rs:83959:    let mut order = Vec::with_capacity(2);
crates/slskr/src/lib.rs:84153:            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
crates/slskr/src/lib.rs:84232:    let mut bound_address_and_port = vec![0_u8; address_len + 2];
crates/slskr/src/lib.rs:91163:    let mut actual = vec![0_u8; HEADER.len()];
crates/slskr/src/controller_tests.rs:820:        vec![0; 12]
crates/slskr/src/controller_tests.rs:2747:        let chunk = vec![b' '; 64 * 1024];
crates/slskr/src/controller_tests.rs:2793:                let chunk = vec![b'x'; 64 * 1024];
crates/slskr/src/controller_tests.rs:8731:    let mut attribute = Vec::with_capacity(8);
crates/slskr/src/controller_tests.rs:8737:    let mut response = Vec::with_capacity(32);
crates/slskr/src/controller_tests.rs:19249:        record.results = vec![template.clone(); super::MAX_SEARCH_RESULTS_PER_SEARCH];
crates/slskr/src/controller_tests.rs:21675:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22051:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22156:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22242:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22407:    assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
crates/slskr/src/controller_tests.rs:24207:        record.members = vec![template.clone(); super::MAX_SHARE_GROUP_MEMBERS];
crates/slskr/src/controller_tests.rs:24369:        record.items = vec![template.clone(); super::MAX_COLLECTION_ITEMS];
crates/slskr/src/controller_tests.rs:28735:        let mut frame = Vec::with_capacity(4 + length as usize);
crates/slskr/src/controller_tests.rs:28850:            let mut actual = vec![0_u8; expected.len()];
crates/slskr/src/controller_tests.rs:103487:        vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103807:        vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103867:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103909:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:104065:            let mut bytes = vec![0_u8; 65_536];
crates/slskr/src/controller_tests.rs:118181:        vec![0_u8; 64 * 1024 + 1],
crates/slskr/src/controller_tests.rs:120121:    let low = entropy.check(&vec![0_u8; EntropyControl::SAMPLE_SIZE]);

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/webhooks.rs:579:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:580:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:759:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:760:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:763:            client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/port_forwarding.rs:84:                "Port {} is already being forwarded",
crates/slskr/src/port_forwarding.rs:101:            bytes_forwarded: Arc::new(AtomicU64::new(0)),
crates/slskr/src/port_forwarding.rs:162:    bytes_forwarded: Arc<AtomicU64>,
crates/slskr/src/port_forwarding.rs:289:        let send_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:310:        let receive_bytes = Arc::clone(&self.bytes_forwarded);
crates/slskr/src/port_forwarding.rs:363:        let bytes_forwarded = self.bytes_forwarded.load(Ordering::Relaxed);
crates/slskr/src/port_forwarding.rs:374:            bytes_forwarded,
crates/slskr/src/port_forwarding.rs:379:            performance: Performance::new(active_connections, bytes_forwarded),
crates/slskr/src/port_forwarding.rs:552:    pub bytes_forwarded: u64,
crates/slskr/src/port_forwarding.rs:912:                if status.bytes_forwarded == 10 {
crates/slskr/src/multisource.rs:656:    let mut builder = Client::builder()
crates/slskr/src/multisource.rs:657:        .redirect(Policy::none())
crates/slskr/src/multisource.rs:661:        builder = builder.resolve(host, SocketAddr::new(address.ip(), port));
crates/slskr/src/relay_agent.rs:259:) -> Result<reqwest::Client, String> {
crates/slskr/src/relay_agent.rs:260:    let mut builder = reqwest::Client::builder()
crates/slskr/src/relay_agent.rs:261:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/relay_agent.rs:704:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:779:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:879:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:923:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:956:    client: &reqwest::Client,
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
crates/slskr/src/private_gateway.rs:272:    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
crates/slskr/src/private_gateway.rs:3030:        .expect("DHT response should be forwarded")
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
crates/slskr/src/application_state.rs:43:        "forwardedPort": runtime.vpn.forwarded_port,
crates/slskr/src/route_dispatch_group_7.rs:2146:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/route_dispatch_group_7.rs:2350:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/cli.rs:2499:    let forwarded = tree
crates/slskr/src/cli.rs:2503:    if forwarded != 1 {
crates/slskr/src/cli.rs:2505:            "distributed search reached {forwarded} children instead of one"
crates/slskr/src/lib.rs:15299:        .to_socket_addrs()
crates/slskr/src/lib.rs:15309:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15311:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:15483:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15485:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:36611:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/lib.rs:36818:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:37921:        let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:37922:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:42633:                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
crates/slskr/src/lib.rs:44416:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44418:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44437:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44439:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44671:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44673:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44725:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44727:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45353:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45355:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45627:        .to_socket_addrs()
crates/slskr/src/lib.rs:45648:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45650:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45687:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45689:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45718:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45720:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45745:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45747:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46565:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46567:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46570:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46607:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46609:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46612:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47367:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47369:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47372:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47501:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47503:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47506:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:48141:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:48143:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48146:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:48321:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:48323:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48678:                .to_socket_addrs()
crates/slskr/src/lib.rs:48695:        .to_socket_addrs()
crates/slskr/src/lib.rs:48732:        .to_socket_addrs()
crates/slskr/src/lib.rs:49054:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/lib.rs:49059:fn forwarded_client_ip(
crates/slskr/src/lib.rs:49064:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/lib.rs:49065:        forwarded_header_client_ips(value)?
crates/slskr/src/lib.rs:49067:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/lib.rs:49068:        x_forwarded_for_client_ips(value)?
crates/slskr/src/lib.rs:49071:    forwarded_ips
crates/slskr/src/lib.rs:49083:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:49086:        .map(parse_forwarded_ip_token)
crates/slskr/src/lib.rs:49091:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:49094:        .map(parse_forwarded_element_ip)
crates/slskr/src/lib.rs:49099:fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:49100:    let mut forwarded_ip = None;
crates/slskr/src/lib.rs:49106:        if forwarded_ip.is_some() {
crates/slskr/src/lib.rs:49109:        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
crates/slskr/src/lib.rs:49111:    forwarded_ip
crates/slskr/src/lib.rs:49114:fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:56413:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:56415:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:56417:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:57586:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:57588:            .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:72371:        reqwest::Client::new().post(endpoint).json(&payload).send(),
crates/slskr/src/lib.rs:73822:                    "primary" => status.forwarded_port,
crates/slskr/src/lib.rs:73877:/// VPN's forwarded port. The local listener remains bound to the configured
crates/slskr/src/lib.rs:73884:            .forwarded_port
crates/slskr/src/lib.rs:85247:            reqwest::Client::new().post(endpoint).json(&payload).send(),
crates/slskr/src/controller_tests.rs:2882:fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
crates/slskr/src/controller_tests.rs:2892:        x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
crates/slskr/src/controller_tests.rs:2897:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3368:fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
crates/slskr/src/controller_tests.rs:3374:        forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
crates/slskr/src/controller_tests.rs:3379:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3385:fn forwarded_ip_parser_rejects_malformed_authorities() {
crates/slskr/src/controller_tests.rs:3398:            super::parse_forwarded_ip_token(malformed),
crates/slskr/src/controller_tests.rs:3404:        super::parse_forwarded_ip_token("\"[2001:db8::42]:443\""),
crates/slskr/src/controller_tests.rs:3408:        super::parse_forwarded_ip_token("198.51.100.24:443"),
crates/slskr/src/controller_tests.rs:3415:fn forwarded_elements_require_one_valid_for_parameter() {
crates/slskr/src/controller_tests.rs:3418:        super::parse_forwarded_element_ip("proto=https; for=198.51.100.24; by=10.0.0.2"),
crates/slskr/src/controller_tests.rs:3429:            super::parse_forwarded_element_ip(malformed),
crates/slskr/src/controller_tests.rs:3444:        x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
crates/slskr/src/controller_tests.rs:3449:        .expect("forwarded client address");
crates/slskr/src/controller_tests.rs:3465:        x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
crates/slskr/src/controller_tests.rs:3476:fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
crates/slskr/src/controller_tests.rs:3482:        forwarded: Some("for=unknown".to_owned()),
crates/slskr/src/controller_tests.rs:3483:        x_forwarded_for: Some("203.0.113.99".to_owned()),
crates/slskr/src/controller_tests.rs:6220:        forwarded_port: Some(44_444),
crates/slskr/src/controller_tests.rs:6243:            "forwardedPort": 44444,
crates/slskr/src/controller_tests.rs:93819:            forwarded_port: Some(44_499),
crates/slskr/src/controller_tests.rs:93844:                && application["vpn"]["forwardedPort"] == 44_499
crates/slskr/src/controller_tests.rs:99333:        let client = reqwest::Client::new();

## Filesystem and persistent-state boundaries
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
crates/slskr/src/ftp.rs:700:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:724:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:731:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:754:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:762:            tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:794:            tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:807:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:820:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:832:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:839:        tokio::fs::create_dir_all(&album).await.unwrap();
crates/slskr/src/ftp.rs:860:        tokio::fs::remove_dir_all(root).await.unwrap();
crates/slskr/src/ftp.rs:899:        tokio::fs::remove_file(file).await.unwrap();
crates/slskr/src/http_server.rs:1727:        std::fs::remove_file(path).unwrap();
crates/slskr/src/http_server.rs:1769:        std::fs::remove_file(path).unwrap();
crates/slskr/src/relay_agent.rs:728:    fs::create_dir_all(&relay_directory)
crates/slskr/src/relay_agent.rs:762:    let cleanup = fs::remove_file(&database_path).await;
crates/slskr/src/relay_agent.rs:1038:    fs::rename(&temporary, &destination)
crates/slskr/src/relay_agent.rs:1083:            match std::fs::remove_file(&self.path) {
crates/slskr/src/private_gateway.rs:2692:    fs::create_dir_all(state_dir)
crates/slskr/src/private_gateway.rs:2718:        return match fs::remove_file(certificate_path) {
crates/slskr/src/private_gateway.rs:2747:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2790:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2802:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2807:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2810:    if let Err(error) = fs::remove_file(&temporary) {
crates/slskr/src/private_gateway.rs:2828:        fs::create_dir_all(&path).unwrap();
crates/slskr/src/private_gateway.rs:3125:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3151:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3180:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3189:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3203:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3218:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3229:        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
crates/slskr/src/private_gateway.rs:3234:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3250:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3264:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3283:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/content_discovery.rs:977:    let mut options = fs::OpenOptions::new();
crates/slskr/src/content_discovery.rs:1336:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1360:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/pod_channels.rs:371:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pod_channels.rs:466:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:488:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:497:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:522:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:531:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:556:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:565:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:589:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/persistence.rs:21:    let file = OpenOptions::new()
crates/slskr/src/persistence.rs:34:    file.set_permissions(std::fs::Permissions::from_mode(0o600))
crates/slskr/src/persistence.rs:5655:        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();
crates/slskr/src/persistence.rs:5671:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/scripts.rs:97:    tokio::fs::create_dir_all(script_directory)
crates/slskr/src/scripts.rs:230:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:254:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:311:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/mesh_security.rs:1044:                fs::create_dir_all(&mesh_directory)
crates/slskr/src/route_dispatch_group_2.rs:3301:    match tokio::fs::remove_file(path).await {
crates/slskr/src/relay.rs:1231:        let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1246:        fs::rename(&temporary_path, &manifest_path)
crates/slskr/src/relay.rs:1251:        let _ = fs::remove_file(&temporary_path);
crates/slskr/src/relay.rs:1467:            tokio::fs::remove_file(path)
crates/slskr/src/relay.rs:1478:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1509:        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
crates/slskr/src/relay.rs:1519:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1541:        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
crates/slskr/src/relay.rs:1551:        std::fs::create_dir_all(&incoming).expect("create concurrent manifest directory");
crates/slskr/src/relay.rs:1600:        std::fs::remove_dir_all(root).expect("remove concurrent manifest fixture");
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
crates/slskr/src/focused_controller_tests.rs:1040:    fs::create_dir_all(managed_file.parent().expect("managed file parent"))
crates/slskr/src/focused_controller_tests.rs:1297:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/focused_controller_tests.rs:1301:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/focused_controller_tests.rs:1432:    fs::create_dir_all(root.join("legacy")).expect("create legacy profile root");
crates/slskr/src/focused_controller_tests.rs:1433:    fs::create_dir_all(root.join("native")).expect("create native profile root");
crates/slskr/src/focused_controller_tests.rs:1472:    let _ = fs::remove_dir_all(root);
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
crates/slskr/src/storage.rs:106:    OpenOptions::new()
crates/slskr/src/multisource.rs:72:            let _ = fs::remove_file(&self.path);
crates/slskr/src/multisource.rs:94:        let _ = fs::remove_dir_all(&self.path);
crates/slskr/src/multisource.rs:474:    fs::create_dir_all(parent).map_err(|_| "output directory could not be created".to_owned())?;
crates/slskr/src/multisource.rs:603:        let _ = fs::remove_file(&assembly_path);
crates/slskr/src/multisource.rs:792:    fs::remove_file(assembly_path)
crates/slskr/src/multisource.rs:821:    let mut options = fs::OpenOptions::new();
crates/slskr/src/multisource.rs:1152:        fs::remove_dir_all(root).expect("remove permissions test root");
crates/slskr/src/multisource.rs:1224:        fs::remove_dir_all(root).expect("remove swarm test root");
crates/slskr/src/multisource.rs:1293:        fs::remove_dir_all(root).expect("remove swarm cancellation test root");
crates/slskr/src/multisource.rs:1322:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/multisource.rs:1383:        fs::remove_dir_all(root).expect("remove mesh preview test root");
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
crates/slskr/src/config.rs:1974:    let file = fs::OpenOptions::new()
crates/slskr/src/config.rs:1980:    fs::remove_file(&probe).map_err(|_| format!("{field} writeability probe cleanup failed"))?;
crates/slskr/src/config.rs:8496:    let mut options = fs::OpenOptions::new();
crates/slskr/src/config.rs:9436:    let mut options = fs::OpenOptions::new();
crates/slskr/src/config.rs:11560:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11576:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11619:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11648:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11696:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11721:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11768:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11796:        std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config.rs:11809:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11853:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11872:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11885:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:11912:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:11999:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12036:        std::fs::remove_file(root.join("slskd.yml")).unwrap();
crates/slskr/src/config.rs:12055:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12108:            std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12125:            std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12136:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12155:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12245:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12300:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12622:        std::fs::create_dir_all(&yaml_downloads).unwrap();
crates/slskr/src/config.rs:12623:        std::fs::create_dir_all(&yaml_incomplete).unwrap();
crates/slskr/src/config.rs:12624:        std::fs::create_dir_all(&yaml_share_a).unwrap();
crates/slskr/src/config.rs:12625:        std::fs::create_dir_all(&yaml_share_b).unwrap();
crates/slskr/src/config.rs:12626:        std::fs::create_dir_all(&env_downloads).unwrap();
crates/slskr/src/config.rs:12688:        std::fs::create_dir_all(&relative_root).unwrap();
crates/slskr/src/config.rs:12718:        std::fs::remove_dir_all(relative_root).unwrap();
crates/slskr/src/config.rs:12719:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12732:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12775:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:12788:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12873:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12886:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12923:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12936:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:12977:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:12990:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13039:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13053:        std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/config.rs:13073:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13086:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13108:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13124:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13135:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13136:        let _ = std::fs::remove_file(outside);
crates/slskr/src/config.rs:13157:        let _ = std::fs::remove_file(path);
crates/slskr/src/config.rs:13176:        let _ = std::fs::remove_dir(path);
crates/slskr/src/config.rs:13192:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13201:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/config.rs:13219:        let _ = std::fs::remove_file(path);
crates/slskr/src/config.rs:13790:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13824:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:13837:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:13864:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14486:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14536:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14601:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14602:        std::fs::create_dir_all(&content).unwrap();
crates/slskr/src/config.rs:14699:        std::fs::remove_dir_all(&content).unwrap();
crates/slskr/src/config.rs:14700:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14727:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14796:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:14825:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:14996:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:15005:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15020:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:15090:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15222:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/config.rs:15231:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/config.rs:15246:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/lib.rs:6501:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6527:                file.set_permissions(fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:6535:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6549:            fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:12441:        let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:14130:            types: canonicalize(
crates/slskr/src/lib.rs:14143:            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
crates/slskr/src/lib.rs:14144:            statuses: canonicalize(
crates/slskr/src/lib.rs:15458:    let _ = fs::remove_file(&normalized_path);
crates/slskr/src/lib.rs:16346:        match existing.canonicalize() {
crates/slskr/src/lib.rs:16392:    let writable = fs::OpenOptions::new()
crates/slskr/src/lib.rs:16398:        let _ = fs::remove_file(probe);
crates/slskr/src/lib.rs:17039:            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
crates/slskr/src/lib.rs:17717:            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
crates/slskr/src/lib.rs:18772:        fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:24443:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:24474:              if remove_file { if let Some(path) = target.local_path.as_deref() { let _ = fs::remove_file(path); } }
crates/slskr/src/lib.rs:37209:        .canonicalize()
crates/slskr/src/lib.rs:37238:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/lib.rs:37261:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/lib.rs:37367:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:37414:    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:37415:    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:40010:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:42827:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:44303:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:44396:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:44404:    match fs::remove_file(path) {
crates/slskr/src/lib.rs:47463:    let directory = fs::canonicalize(directory)
crates/slskr/src/lib.rs:47471:        fs::remove_file(&path).map_err(|error| {
crates/slskr/src/lib.rs:51020:                                    let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:51026:                            let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:51042:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:51053:    if let Err(error) = fs::remove_file(path) {
crates/slskr/src/lib.rs:70591:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:70608:            .canonicalize()
crates/slskr/src/lib.rs:70615:                .canonicalize()
crates/slskr/src/lib.rs:70620:                .canonicalize()
crates/slskr/src/lib.rs:72303:        fs::remove_file(path)
crates/slskr/src/lib.rs:72307:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:72312:    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
crates/slskr/src/lib.rs:73423:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/lib.rs:73435:        std::fs::create_dir_all(path)
crates/slskr/src/lib.rs:73482:    std::fs::create_dir_all(path).map_err(|error| {
crates/slskr/src/lib.rs:74680:    let _ = fs::remove_file(output_path);
crates/slskr/src/lib.rs:77547:        let canonical_path = local_path.canonicalize().ok()?;
crates/slskr/src/lib.rs:77551:            .filter_map(|root| root.canonicalize().ok())
crates/slskr/src/lib.rs:77567:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:77861:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:77866:        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
crates/slskr/src/lib.rs:77884:    let file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:77892:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77957:                let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:77969:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:78268:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:78275:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:78862:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:78870:            .canonicalize()
crates/slskr/src/lib.rs:78872:        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
crates/slskr/src/lib.rs:78892:            fs::remove_dir_all(&path)
crates/slskr/src/lib.rs:78898:            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/lib.rs:79038:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/lib.rs:79046:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:79050:        .canonicalize()
crates/slskr/src/lib.rs:79055:        .canonicalize()
crates/slskr/src/lib.rs:79147:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:79198:        .canonicalize()
crates/slskr/src/lib.rs:79201:        .canonicalize()
crates/slskr/src/lib.rs:79206:    fs::OpenOptions::new()
crates/slskr/src/lib.rs:82859:        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
crates/slskr/src/lib.rs:82871:            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
crates/slskr/src/lib.rs:83535:            fs::OpenOptions::new()
crates/slskr/src/lib.rs:83601:        fs::rename(&final_path, &incomplete_path)
crates/slskr/src/lib.rs:83629:        fs::remove_file(&completed_path)
crates/slskr/src/lib.rs:83632:    match fs::rename(&incomplete_path, &completed_path) {
crates/slskr/src/lib.rs:83640:            fs::remove_file(&incomplete_path)
crates/slskr/src/lib.rs:83752:        fs::create_dir_all(&root)
crates/slskr/src/lib.rs:83759:        fs::rename(path, destination)
crates/slskr/src/lib.rs:83762:        fs::remove_file(path)
crates/slskr/src/lib.rs:85205:        match tokio::fs::create_dir_all(&log_dir).await {
crates/slskr/src/lib.rs:85207:                match tokio::fs::OpenOptions::new()
crates/slskr/src/lib.rs:87904:            let _ = fs::remove_dir(&path);
crates/slskr/src/lib.rs:87907:            let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:88079:                let _ = fs::remove_file(entry.path());
crates/slskr/src/lib.rs:89126:                let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:90788:        match root.canonicalize() {
crates/slskr/src/lib.rs:90877:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/lib.rs:91122:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:91131:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:91187:        fs::remove_file(&rotated_path)
crates/slskr/src/lib.rs:91190:    fs::rename(path, &rotated_path)
crates/slskr/src/lib.rs:91217:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:91297:    fs::create_dir_all(parent)?;
crates/slskr/src/lib.rs:91322:        let mut file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:91333:            let _ = fs::remove_file(temp_path);
crates/slskr/src/lib.rs:91351:    fs::rename(source, destination)
crates/slskr/src/lib.rs:91359:    match fs::remove_file(destination) {
crates/slskr/src/lib.rs:91364:    fs::rename(source, destination)
crates/slskr/src/lib.rs:91394:    let mut options = fs::OpenOptions::new();
crates/slskr/src/controller_tests.rs:79:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:117:        fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:299:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:334:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:348:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:369:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:383:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:398:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:695:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:719:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:734:    fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:805:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:811:    fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:1310:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1403:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:1413:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1461:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:1694:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:2319:    fs::create_dir_all(&evidence_dir).expect("create server/session evidence directory");
crates/slskr/src/controller_tests.rs:2682:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:3519:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:4011:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4108:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4382:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4463:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4640:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:5059:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5228:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5331:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5975:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:5998:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6006:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6098:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6106:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6193:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6262:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6326:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6894:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7306:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7333:    std::fs::create_dir_all(&root).expect("gateway state directory");
crates/slskr/src/controller_tests.rs:7897:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7905:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:8722:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:9396:    std::fs::create_dir_all(root.join("assets")).unwrap();
crates/slskr/src/controller_tests.rs:9397:    std::fs::create_dir_all(root.join("static")).unwrap();
crates/slskr/src/controller_tests.rs:9432:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9457:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9466:    std::fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:9475:    let _ = std::fs::remove_file(outside);
crates/slskr/src/controller_tests.rs:9476:    let _ = std::fs::remove_dir_all(outside_dir);
crates/slskr/src/controller_tests.rs:9477:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9498:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9542:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9556:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9573:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9816:    std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:9949:    std::fs::create_dir_all(&album).unwrap();
crates/slskr/src/controller_tests.rs:10043:    std::fs::create_dir_all(&dir).unwrap();
crates/slskr/src/controller_tests.rs:10081:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10082:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:10115:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10151:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10185:        std::fs::create_dir_all(&directory).unwrap();
crates/slskr/src/controller_tests.rs:11670:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:11804:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12206:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12609:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12716:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13063:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13068:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13317:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13322:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14041:    fs::create_dir_all(&evidence_dir).expect("create application evidence directory");
crates/slskr/src/controller_tests.rs:14082:    std::fs::create_dir_all(&root).expect("share root");
crates/slskr/src/controller_tests.rs:14130:    std::fs::remove_dir_all(root).expect("remove share root");
crates/slskr/src/controller_tests.rs:14326:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14508:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14745:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:16819:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19432:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19647:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20020:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20237:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20580:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20662:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:21100:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21199:    std::fs::create_dir_all(parent).expect("download parent dir");
crates/slskr/src/controller_tests.rs:21209:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21228:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21229:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21236:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21255:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21256:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21265:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21283:    std::fs::create_dir_all(&dir).expect("test dir");
crates/slskr/src/controller_tests.rs:21289:    std::fs::remove_file(&shared_path).expect("remove shared file");
crates/slskr/src/controller_tests.rs:21299:    let _ = std::fs::remove_dir_all(dir);
crates/slskr/src/controller_tests.rs:21320:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:21321:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:21332:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:21333:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:21372:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21695:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21715:    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
crates/slskr/src/controller_tests.rs:21801:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21875:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21966:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22071:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22174:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22265:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22415:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:26493:    std::fs::create_dir_all(&root).expect("create stream share root");
crates/slskr/src/controller_tests.rs:26561:    std::fs::remove_dir_all(root).expect("remove stream fixture");
crates/slskr/src/controller_tests.rs:26596:    std::fs::create_dir_all(&root).expect("create preview share root");
crates/slskr/src/controller_tests.rs:26660:    std::fs::remove_dir_all(root).expect("remove preview fixture");
crates/slskr/src/controller_tests.rs:26976:    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
crates/slskr/src/controller_tests.rs:27064:    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");
crates/slskr/src/controller_tests.rs:27067:    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
crates/slskr/src/controller_tests.rs:27068:    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
crates/slskr/src/controller_tests.rs:27069:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:27240:    std::fs::create_dir_all(&child).unwrap();
crates/slskr/src/controller_tests.rs:27266:        std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:27273:        std::fs::remove_dir_all(outside).unwrap();
crates/slskr/src/controller_tests.rs:27276:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:27424:    let _ = std::fs::remove_file(&queue.state_path);
crates/slskr/src/controller_tests.rs:27425:    let _ = std::fs::remove_file(&queue.events_path);
crates/slskr/src/controller_tests.rs:27897:    fs::create_dir_all(&root).expect("create overlay search state directory");
crates/slskr/src/controller_tests.rs:28022:    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
crates/slskr/src/controller_tests.rs:28032:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28153:    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
crates/slskr/src/controller_tests.rs:28400:    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
crates/slskr/src/controller_tests.rs:28406:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28705:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:28940:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:29116:    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
crates/slskr/src/controller_tests.rs:29259:    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
crates/slskr/src/controller_tests.rs:29708:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:29884:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:31475:    fs::create_dir_all(&config.downloads_dir).unwrap();
crates/slskr/src/controller_tests.rs:31484:    fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:31495:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:31539:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:32002:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:32148:    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32425:    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
crates/slskr/src/controller_tests.rs:32486:    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
crates/slskr/src/controller_tests.rs:32487:    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32822:    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
crates/slskr/src/controller_tests.rs:33076:    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
crates/slskr/src/controller_tests.rs:33316:    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
crates/slskr/src/controller_tests.rs:33328:    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
crates/slskr/src/controller_tests.rs:33431:    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
crates/slskr/src/controller_tests.rs:34317:    std::fs::create_dir_all(&root).expect("create listening-party share root");
crates/slskr/src/controller_tests.rs:34408:    std::fs::remove_dir_all(root).expect("remove listening-party fixture");
crates/slskr/src/controller_tests.rs:35017:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35201:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35332:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35644:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35796:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35999:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:36540:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39174:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39255:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39430:    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
crates/slskr/src/controller_tests.rs:39458:    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
crates/slskr/src/controller_tests.rs:40751:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:40762:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:42069:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:42959:    fs::create_dir_all(root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:43008:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:43172:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:43278:        let _ = fs::remove_file(path);
crates/slskr/src/controller_tests.rs:43281:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:43885:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:44184:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45794:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45898:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:47099:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47247:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47437:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47653:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47854:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48111:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48404:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49125:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49437:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49822:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50031:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50071:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:50136:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:50142:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50475:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50662:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51036:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:51285:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51748:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52804:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53064:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53210:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53971:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54260:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:54436:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54610:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54676:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54758:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54827:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:55102:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55433:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55900:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56281:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56387:        fs::remove_file(&pods_path).expect("remove channel create state file");
crates/slskr/src/controller_tests.rs:56413:        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
crates/slskr/src/controller_tests.rs:56500:        fs::remove_file(&pods_path).expect("remove channel update state file");
crates/slskr/src/controller_tests.rs:56533:        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
crates/slskr/src/controller_tests.rs:56621:        fs::remove_file(&pods_path).expect("remove channel delete state file");
crates/slskr/src/controller_tests.rs:56647:        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
crates/slskr/src/controller_tests.rs:56725:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:56915:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57154:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57293:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57484:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57683:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57779:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58075:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58617:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59006:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59350:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59791:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60116:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60383:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60502:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60645:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61413:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61650:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61877:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62065:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62158:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62308:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62491:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62772:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63222:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63368:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63545:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:63801:    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
crates/slskr/src/controller_tests.rs:63935:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64359:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:64556:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64930:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65075:    fs::create_dir_all(&evidence_dir).expect("create discovery graph edge evidence directory");
crates/slskr/src/controller_tests.rs:65356:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65601:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66101:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66476:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66804:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67386:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67701:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:67929:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68120:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68493:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68929:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:69382:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:70207:    fs::create_dir_all(&evidence_dir).expect("create quarantine-jury evidence directory");
crates/slskr/src/controller_tests.rs:70446:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:70980:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:71585:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:71866:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72492:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72842:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73283:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73619:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73730:            fs::remove_file(&path).expect("remove message storage file");
crates/slskr/src/controller_tests.rs:73894:        fs::remove_dir(&messages_path).expect("remove blocked global message path");
crates/slskr/src/controller_tests.rs:74046:        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
crates/slskr/src/controller_tests.rs:74072:        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
crates/slskr/src/controller_tests.rs:74103:        fs::remove_dir(&messages_path).expect("remove blocked search message path");
crates/slskr/src/controller_tests.rs:74154:        fs::remove_dir(&messages_path).expect("remove blocked count message path");
crates/slskr/src/controller_tests.rs:74291:            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
crates/slskr/src/controller_tests.rs:74298:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74401:            fs::remove_file(&path).expect("remove membership storage file");
crates/slskr/src/controller_tests.rs:74444:        fs::remove_dir(&pods_path).expect("remove blocked membership delete path");
crates/slskr/src/controller_tests.rs:74533:        fs::remove_dir(&pods_path).expect("remove blocked membership projection path");
crates/slskr/src/controller_tests.rs:74552:        fs::remove_dir(&pods_path).expect("remove blocked membership stats path");
crates/slskr/src/controller_tests.rs:74605:        fs::remove_dir(&pods_path).expect("remove blocked membership moderation path");
crates/slskr/src/controller_tests.rs:74700:        fs::remove_dir(&pods_path).expect("remove blocked membership publish path");
crates/slskr/src/controller_tests.rs:74784:        fs::remove_dir(&pods_path).expect("remove blocked membership update path");
crates/slskr/src/controller_tests.rs:74867:        fs::remove_dir(&pods_path).expect("remove blocked membership cleanup path");
crates/slskr/src/controller_tests.rs:74896:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74964:                fs::remove_file(&path).expect("remove discovery feature state file");
crates/slskr/src/controller_tests.rs:75073:        fs::remove_dir(&feature_path).expect("remove blocked discovery registration path");
crates/slskr/src/controller_tests.rs:75161:        fs::remove_dir(&feature_path).expect("remove blocked discovery update path");
crates/slskr/src/controller_tests.rs:75274:        fs::remove_dir(&feature_path).expect("remove blocked discovery unregister path");
crates/slskr/src/controller_tests.rs:75406:        fs::remove_dir(&feature_path).expect("remove blocked discovery projection path");
crates/slskr/src/controller_tests.rs:75466:        fs::remove_dir(&feature_path).expect("remove blocked discovery refresh path");
crates/slskr/src/controller_tests.rs:75555:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:76375:    fs::create_dir_all(&evidence_dir).expect("create PodJoinLeave evidence directory");
crates/slskr/src/controller_tests.rs:76846:    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
crates/slskr/src/controller_tests.rs:77293:    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
crates/slskr/src/controller_tests.rs:78153:    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
crates/slskr/src/controller_tests.rs:78865:    fs::create_dir_all(&evidence_dir).expect("create MultiSource evidence directory");
crates/slskr/src/controller_tests.rs:79280:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79422:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79678:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79893:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80158:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80385:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80416:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81470:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81729:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:82546:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:83290:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83594:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83854:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84155:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84360:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84566:                    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:84659:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84777:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84985:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:84990:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85110:    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:85297:    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:85302:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85492:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85836:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86085:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86162:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86260:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86350:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86570:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86749:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86851:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86914:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86984:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87026:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87078:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87133:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87456:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87633:    let _ = fs::remove_file(&validation_path);
crates/slskr/src/controller_tests.rs:87796:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88050:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88182:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:88287:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88468:    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
crates/slskr/src/controller_tests.rs:88687:    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
crates/slskr/src/controller_tests.rs:88847:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88939:    std::fs::create_dir_all(download_file.parent().unwrap())
crates/slskr/src/controller_tests.rs:88997:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:89145:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89231:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89334:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89453:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89505:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90028:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90400:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90469:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90516:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90566:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90620:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90724:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90781:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90842:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90887:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90943:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91000:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91117:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91178:    fs::create_dir_all(&custom_path).expect("create destination fixture");
crates/slskr/src/controller_tests.rs:91235:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91239:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91294:    fs::create_dir_all(&root).expect("create destination edge root");
crates/slskr/src/controller_tests.rs:91528:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91535:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91775:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92294:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93017:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93171:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93411:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:93680:        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
crates/slskr/src/controller_tests.rs:93735:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:93741:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93971:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94041:        std::fs::create_dir_all(&root).expect("create differential downloads root");
crates/slskr/src/controller_tests.rs:94072:        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
crates/slskr/src/controller_tests.rs:94123:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94590:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94801:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94904:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95388:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95618:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95779:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96437:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96974:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:97884:    fs::create_dir_all(existing.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:98113:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98604:    fs::create_dir_all(&new_root).unwrap();
crates/slskr/src/controller_tests.rs:98605:    fs::create_dir_all(&new_downloads).unwrap();
crates/slskr/src/controller_tests.rs:98606:    fs::create_dir_all(&new_incomplete).unwrap();
crates/slskr/src/controller_tests.rs:99004:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99035:        fs::create_dir_all(download_file.parent().unwrap()).expect("downloads fixture root");
crates/slskr/src/controller_tests.rs:99036:        fs::create_dir_all(incomplete_file.parent().unwrap()).expect("incomplete fixture root");
crates/slskr/src/controller_tests.rs:99169:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99274:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99492:        let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:99498:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99522:        fs::create_dir_all(&root).expect("secure writer root");
crates/slskr/src/controller_tests.rs:99586:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:99592:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99614:    fs::create_dir_all(&root).expect("DHT certificate root");
crates/slskr/src/controller_tests.rs:99647:        fs::create_dir_all(&linked_root).expect("DHT symlink root");
crates/slskr/src/controller_tests.rs:99705:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99712:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100641:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100666:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100667:    let _ = std::fs::remove_file(&outside);
crates/slskr/src/controller_tests.rs:100692:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100730:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100849:    std::fs::create_dir_all(&nested).expect("create nested dir");
crates/slskr/src/controller_tests.rs:100866:    std::fs::create_dir_all(&album).expect("create recursive directory");
crates/slskr/src/controller_tests.rs:100875:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100896:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100897:    std::fs::create_dir_all(&outside).expect("create outside");
crates/slskr/src/controller_tests.rs:100910:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100911:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:100928:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100943:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100963:    std::fs::create_dir_all(&directory).expect("create deep directory tree");
crates/slskr/src/controller_tests.rs:100973:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101670:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101677:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101691:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101697:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101756:    std::fs::create_dir_all(&artist).unwrap();
crates/slskr/src/controller_tests.rs:101758:    std::fs::create_dir_all(root.join(".hidden")).unwrap();
crates/slskr/src/controller_tests.rs:101775:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101783:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101820:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101830:    std::fs::create_dir_all(&first).unwrap();
crates/slskr/src/controller_tests.rs:101831:    std::fs::create_dir_all(&second).unwrap();
crates/slskr/src/controller_tests.rs:101844:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101883:    std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/controller_tests.rs:101904:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101928:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101941:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101962:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101963:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:101977:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101978:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:102017:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:102037:    std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:102053:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102335:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102336:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102843:    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
crates/slskr/src/controller_tests.rs:102917:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:102956:    let _ = std::fs::remove_file(&path);
crates/slskr/src/controller_tests.rs:102957:    let mut file = std::fs::OpenOptions::new()
crates/slskr/src/controller_tests.rs:102974:    std::fs::remove_file(path).expect("remove cancelled transfer test file");
crates/slskr/src/controller_tests.rs:103015:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103016:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103054:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103055:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103074:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103075:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103124:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103125:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103188:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103189:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103241:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103242:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103305:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103306:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103320:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103357:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103371:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103438:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103483:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103494:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103509:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103521:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103538:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103605:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103619:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103632:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103646:    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
crates/slskr/src/controller_tests.rs:103755:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:103762:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103777:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103789:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103803:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103859:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103892:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:103901:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:104458:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:104893:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:104898:    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
crates/slskr/src/controller_tests.rs:105069:    std::fs::remove_file(picture).unwrap();
crates/slskr/src/controller_tests.rs:105262:    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
crates/slskr/src/controller_tests.rs:105264:    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
crates/slskr/src/controller_tests.rs:105359:        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:105625:        fs::create_dir_all(&downloads_target).expect("create downloads list target");
crates/slskr/src/controller_tests.rs:105626:        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
crates/slskr/src/controller_tests.rs:105661:        let _ = fs::remove_file(downloads_link);
crates/slskr/src/controller_tests.rs:105662:        let _ = fs::remove_file(incomplete_link);
crates/slskr/src/controller_tests.rs:105663:        let _ = fs::remove_dir_all(downloads_target);
crates/slskr/src/controller_tests.rs:105664:        let _ = fs::remove_dir_all(incomplete_target);
crates/slskr/src/controller_tests.rs:105666:    let _ = fs::remove_file(downloads_conflict_root);
crates/slskr/src/controller_tests.rs:105667:    let _ = fs::remove_file(incomplete_conflict_root);
crates/slskr/src/controller_tests.rs:105920:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:105962:    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
crates/slskr/src/controller_tests.rs:106214:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106485:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106565:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106899:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106939:    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
crates/slskr/src/controller_tests.rs:106940:    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
crates/slskr/src/controller_tests.rs:107206:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:107294:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:107333:    fs::remove_file(downloads_root.join("Relay/Agent.txt"))
crates/slskr/src/controller_tests.rs:107467:    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
crates/slskr/src/controller_tests.rs:107510:    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
crates/slskr/src/controller_tests.rs:107511:    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");
crates/slskr/src/controller_tests.rs:107636:    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
crates/slskr/src/controller_tests.rs:107678:    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
crates/slskr/src/controller_tests.rs:107679:    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
crates/slskr/src/controller_tests.rs:107680:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:107681:    let _ = fs::remove_dir_all(downloads_root);
crates/slskr/src/controller_tests.rs:107686:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108634:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108958:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109297:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109766:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110511:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110746:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111034:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111458:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111707:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112615:    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
crates/slskr/src/controller_tests.rs:112873:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113183:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113711:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113990:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114389:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114810:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115188:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115399:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115691:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116120:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116366:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116640:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117159:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117379:    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
crates/slskr/src/controller_tests.rs:117428:        fs::create_dir_all(&root).expect("path guard root");
crates/slskr/src/controller_tests.rs:117516:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:117598:    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
crates/slskr/src/controller_tests.rs:117701:    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
crates/slskr/src/controller_tests.rs:117864:    fs::create_dir_all(&evidence_dir).expect("create CSRF security evidence directory");
crates/slskr/src/controller_tests.rs:117993:    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
crates/slskr/src/controller_tests.rs:118007:    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");
crates/slskr/src/controller_tests.rs:118055:    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
crates/slskr/src/controller_tests.rs:118102:    fs::create_dir_all(&root).expect("certificate manager root");
crates/slskr/src/controller_tests.rs:118161:    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
crates/slskr/src/controller_tests.rs:118178:    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
crates/slskr/src/controller_tests.rs:118201:        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
crates/slskr/src/controller_tests.rs:118266:    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
crates/slskr/src/controller_tests.rs:118273:    fs::remove_dir_all(&root).expect("remove certificate manager root");
crates/slskr/src/controller_tests.rs:118441:    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
crates/slskr/src/controller_tests.rs:118587:    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
crates/slskr/src/controller_tests.rs:118954:    fs::create_dir_all(&certificate_root).expect("certificate root");
crates/slskr/src/controller_tests.rs:118983:    fs::create_dir_all(&malformed_root).expect("malformed certificate root");
crates/slskr/src/controller_tests.rs:119012:    let _ = fs::remove_dir_all(&certificate_root);
crates/slskr/src/controller_tests.rs:119013:    let _ = fs::remove_dir_all(&malformed_root);
crates/slskr/src/controller_tests.rs:119018:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:119072:    fs::create_dir_all(&root).expect("content-safety root");
crates/slskr/src/controller_tests.rs:119151:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:119155:    fs::create_dir_all(&evidence_dir).expect("create content-safety evidence directory");
crates/slskr/src/controller_tests.rs:119274:    fs::create_dir_all(&evidence_dir).expect("create Soulseek safety evidence directory");
crates/slskr/src/controller_tests.rs:119398:    fs::create_dir_all(&evidence_dir).expect("create security event sink evidence directory");
crates/slskr/src/controller_tests.rs:119944:    std::fs::create_dir_all(&evidence_dir).expect("create integrity evidence directory");
crates/slskr/src/controller_tests.rs:120623:    std::fs::create_dir_all(&evidence_dir).expect("create runtime-control evidence directory");
crates/slskr/src/controller_tests.rs:120833:    std::fs::create_dir_all(&evidence_dir).expect("create route-security evidence directory");
crates/slskr/src/controller_tests.rs:121232:    let _ = fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:121530:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121709:    fs::create_dir_all(&root).expect("JWT revocation root");
crates/slskr/src/controller_tests.rs:121761:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121766:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121887:    fs::create_dir_all(&evidence_dir).expect("create security controller evidence directory");
crates/slskr/src/controller_tests.rs:121971:    fs::create_dir_all(&evidence_dir).expect("create passthrough security evidence directory");
crates/slskr/src/controller_tests.rs:122026:        fs::create_dir_all(&root).expect("authentication control state root");
crates/slskr/src/controller_tests.rs:122185:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122191:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:122239:    fs::create_dir_all(&root).expect("pin file lifecycle root");
crates/slskr/src/controller_tests.rs:122281:        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
crates/slskr/src/controller_tests.rs:122305:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122312:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122331:    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
crates/slskr/src/controller_tests.rs:122378:        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
crates/slskr/src/controller_tests.rs:122402:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122409:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122554:    fs::create_dir_all(&root).expect("create multisource lifecycle root");
crates/slskr/src/controller_tests.rs:122830:    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
crates/slskr/src/controller_tests.rs:122839:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:123084:        let _ = fs::remove_file(yaml_failure_root);
crates/slskr/src/controller_tests.rs:123256:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:123721:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:124266:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:124444:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124514:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124672:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124727:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124888:        let _ = std::fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124938:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125181:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125319:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125463:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125520:    fs::create_dir_all(&evidence_dir).expect("create SongID persistence evidence directory");
crates/slskr/src/controller_tests.rs:125626:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125664:    fs::create_dir_all(&evidence_dir).expect("create TrafficStats evidence directory");
crates/slskr/src/controller_tests.rs:126288:    fs::create_dir_all(&evidence_dir).expect("create HashDb controller evidence directory");
crates/slskr/src/controller_tests.rs:126384:            fs::remove_file(&path).expect("remove state file before runtime failure");
crates/slskr/src/controller_tests.rs:127601:    fs::create_dir_all(&evidence_dir).expect("create PodsController evidence directory");
crates/slskr/src/controller_tests.rs:128878:    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
crates/slskr/src/controller_tests.rs:129226:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:130229:    fs::create_dir_all(&evidence_dir).expect("create RoomsController evidence directory");
crates/slskr/src/controller_tests.rs:130966:    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
crates/slskr/src/controller_tests.rs:131039:            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
crates/slskr/src/controller_tests.rs:131056:                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
crates/slskr/src/controller_tests.rs:131058:                fs::remove_file(&path).expect("remove prepared PodCore feature file");
crates/slskr/src/controller_tests.rs:133124:    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
crates/slskr/src/controller_tests.rs:133543:        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
crates/slskr/src/controller_tests.rs:133585:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/controller_tests.rs:133608:    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
crates/slskr/src/controller_tests.rs:134402:    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
crates/slskr/src/controller_tests.rs:134951:    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
crates/slskr/src/controller_tests.rs:135096:    fs::create_dir_all(&item_root).expect("create residual library directory");
crates/slskr/src/controller_tests.rs:135210:    let _ = fs::remove_dir_all(&item_root);
crates/slskr/src/controller_tests.rs:135452:    fs::create_dir_all(&evidence_dir).expect("create Library evidence directory");
crates/slskr/src/controller_tests.rs:136383:    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
crates/slskr/src/controller_tests.rs:136944:        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
crates/slskr/src/controller_tests.rs:137402:    fs::create_dir_all(&evidence_dir).expect("create Integrations evidence directory");
crates/slskr/src/controller_tests.rs:138162:    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
crates/slskr/src/controller_tests.rs:138855:    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
crates/slskr/src/controller_tests.rs:139228:    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
crates/slskr/src/controller_tests.rs:139591:    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
crates/slskr/src/controller_tests.rs:140079:    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
crates/slskr/src/controller_tests.rs:140621:    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
crates/slskr/src/controller_tests.rs:141066:    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
crates/slskr/src/controller_tests.rs:141677:    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
crates/slskr/src/controller_tests.rs:142089:    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
crates/slskr/src/controller_tests.rs:142376:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
crates/slskr/src/controller_tests.rs:142908:    let _ = fs::remove_dir_all(super::effective_downloads_dir(&controller_state));
crates/slskr/src/controller_tests.rs:142909:    let _ = fs::remove_file(share_source);
crates/slskr/src/controller_tests.rs:142914:    fs::create_dir_all(&evidence_dir).expect("create relay evidence directory");
crates/slskr/src/controller_tests.rs:143661:    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
crates/slskr/src/controller_tests.rs:144346:    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
crates/slskr/src/controller_tests.rs:144461:            fs::create_dir_all(&path).expect("create nominal directory");
crates/slskr/src/controller_tests.rs:144524:            fs::create_dir_all(&path).expect("create mutation directory");
crates/slskr/src/controller_tests.rs:144558:            fs::create_dir_all(&path).expect("create concurrent directory");
crates/slskr/src/controller_tests.rs:144596:            fs::create_dir_all(&root).expect("create file storage root");
crates/slskr/src/controller_tests.rs:144648:            fs::create_dir_all(&root).expect("create concurrent file root");
crates/slskr/src/controller_tests.rs:144696:        fs::create_dir_all(&root).expect("create incomplete mutation root");
crates/slskr/src/controller_tests.rs:144775:            fs::create_dir_all(root.join("Album")).expect("create populated root");
crates/slskr/src/controller_tests.rs:144794:            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
crates/slskr/src/controller_tests.rs:144853:            fs::create_dir_all(&album).expect("create populated detail root");
crates/slskr/src/controller_tests.rs:144879:    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");

## Async task and channel lifecycle boundaries
crates/slskr-client/src/quic_data.rs:615:    tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:656:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_data.rs:777:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:824:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:865:        let server = tokio::spawn(async move {
crates/slskr-client/src/transfer.rs:156:        self.receive_file_from_with_timeout(
crates/slskr-client/src/transfer.rs:204:        let result = time::timeout(timeout, async {
crates/slskr-client/src/transfer.rs:451:        self.send_file_to_with_timeout(connection, bytes, DEFAULT_TRANSFER_IO_TIMEOUT)
crates/slskr-client/src/transfer.rs:481:        let result = time::timeout(timeout, async {
crates/slskr-client/src/quic_control.rs:253:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:386:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:403:    tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:452:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:499:        let server = tokio::spawn(async move {
crates/slskr-client/src/peer_cache.rs:125:        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:129:    pub async fn send_to_with_timeout(
crates/slskr-client/src/peer_cache.rs:146:        match time::timeout(timeout, active.send(message)).await {
crates/slskr-client/src/peer_cache.rs:167:        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:171:    pub async fn receive_from_with_timeout(
crates/slskr-client/src/peer_cache.rs:187:        match time::timeout(timeout, active.receive()).await {
crates/slskr-client/src/manager.rs:122:        self.ensure_peer_messages_with_timeout(username, DEFAULT_MANAGER_CONNECT_TIMEOUT)
crates/slskr-client/src/manager.rs:126:    pub async fn ensure_peer_messages_with_timeout(
crates/slskr-client/src/manager.rs:136:        time::timeout(timeout, async {
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
crates/slskr-client/src/distributed_tree.rs:343:        self.send_branch_info_to_parent_with_timeout(DEFAULT_DISTRIBUTED_IO_TIMEOUT)
crates/slskr-client/src/distributed_tree.rs:347:    pub async fn send_branch_info_to_parent_with_timeout(
crates/slskr-client/src/distributed_tree.rs:359:        let result = time::timeout(timeout, async {
crates/slskr-client/src/distributed_tree.rs:385:        self.forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:393:    pub async fn forward_search_to_children_with_timeout(
crates/slskr-client/src/distributed_tree.rs:406:        let result = time::timeout(timeout, async {
crates/slskr-client/src/peer_connect.rs:210:    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:238:    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:266:    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:295:    time::timeout(timeout, future)
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
crates/slskr-client/src/stream.rs:35:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:42:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr/src/mesh_services.rs:407:    timeout(deadline, operation)
crates/slskr/src/mesh_services.rs:553:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:567:        let fetch = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:654:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:668:        let fetch = tokio::spawn(async move {
crates/slskr-client/src/search.rs:75:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:122:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:123:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/search.rs:153:    pub fn set_server_interval(&mut self, seconds: Option<u64>) {
crates/slskr/src/route_dispatch_group_7.rs:1327:                tokio::spawn(multisource::execute(
crates/slskr/src/route_dispatch_group_3.rs:748:                tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_1.rs:501:                let response = tokio::time::timeout(
crates/slskr/src/route_dispatch_group_1.rs:1464:                tokio::spawn(async move {
crates/slskr/src/dht.rs:188:        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
crates/slskr/src/dht.rs:201:                match timeout(
crates/slskr/src/dht.rs:246:        timeout(self.lookup_timeout, async {
crates/slskr/src/scripts.rs:15:fn format_timeout(duration: Duration) -> String {
crates/slskr/src/scripts.rs:87:    run_with_timeout(script, script_directory, target, payload, SCRIPT_TIMEOUT).await
crates/slskr/src/scripts.rs:90:async fn run_with_timeout(
crates/slskr/src/scripts.rs:108:    let output = time::timeout(timeout_duration, command.output())
crates/slskr/src/scripts.rs:113:                format_timeout(timeout_duration)
crates/slskr/src/scripts.rs:167:        tokio::spawn(async move {
crates/slskr/src/scripts.rs:243:        let error = run_with_timeout(
crates/slskr/src/webhooks.rs:605:                .timeout(timeout)
crates/slskr/src/webhooks.rs:669:            tokio::spawn(async move {
crates/slskr/src/webhooks.rs:773:            .timeout(request_timeout)
crates/slskr/src/webhooks.rs:896:    tokio::time::timeout(timeout, resolution)
crates/slskr/src/webhooks.rs:1042:        let server = tokio::spawn(async move {
crates/slskr/src/webhooks.rs:1074:        let server = tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:49:    let handshake = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/relay_ws.rs:104:    let reader_task = tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:106:            let frame = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/relay_ws.rs:114:    let mut keepalive = time::interval(SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_ws.rs:401:    time::timeout(
crates/slskr/src/relay_ws.rs:520:    time::timeout(timeout, read_ws_frame(reader))
crates/slskr/src/relay_ws.rs:532:        let error = time::timeout(
crates/slskr/src/relay_ws.rs:534:            read_ws_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr/src/port_forwarding.rs:110:        let task = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:126:            if timeout(Duration::from_secs(5), &mut task).await.is_err() {
crates/slskr/src/port_forwarding.rs:193:                            tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:292:        let mut send = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:313:        let mut receive = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:345:            match timeout(TUNNEL_CLOSE_TIMEOUT, close_tunnel(&client, &tunnel_id)).await {
crates/slskr/src/port_forwarding.rs:491:    let reply = timeout(SERVICE_CALL_TIMEOUT, async {
crates/slskr/src/port_forwarding.rs:669:        timeout(Duration::from_secs(1), async {
crates/slskr/src/port_forwarding.rs:721:        let stalled_gateway = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:735:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:748:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:815:        let gateway = tokio::spawn(async move {
crates/slskr/src/port_forwarding.rs:904:        timeout(Duration::from_secs(5), local.read_exact(&mut echoed))
crates/slskr/src/port_forwarding.rs:909:        timeout(Duration::from_secs(2), async {
crates/slskr/src/port_forwarding.rs:924:        timeout(Duration::from_secs(5), gateway)
crates/slskr/src/ftp.rs:228:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:236:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:250:            let ftp = tokio::time::timeout(timeout, AsyncRustlsFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:254:            let ftp = tokio::time::timeout(
crates/slskr/src/ftp.rs:272:            if let Ok(Ok(ftp)) = tokio::time::timeout(timeout, secure).await {
crates/slskr/src/ftp.rs:275:            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
crates/slskr/src/ftp.rs:325:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:544:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:579:        let server = tokio::spawn(async move {
crates/slskr/src/ftp.rs:884:            tokio::time::timeout(Duration::from_millis(50), listener.accept())
crates/slskr/src/ftp.rs:889:        let attempted = tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_2.rs:2879:            let interests = match time::timeout(
crates/slskr/src/batch.rs:410:    fn test_batch_rejects_invalid_timeout() {
crates/slskr/src/dotnet_regex.rs:58:    pub fn is_match_with_timeout(&self, value: &str, timeout: Duration) -> Result<bool, String> {
crates/slskr/src/dotnet_regex.rs:76:        match receiver.recv_timeout(timeout) {
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
crates/slskr/src/mesh_sync.rs:354:    let result = tokio::task::spawn_blocking(move || read_file_chunk(path, offset, length)).await;
crates/slskr/src/multisource.rs:659:        .timeout(SOURCE_TIMEOUT);
crates/slskr/src/multisource.rs:699:    timeout(deadline, resolution)
crates/slskr/src/multisource.rs:915:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:921:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:971:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1256:        let download = tokio::spawn(execute(
crates/slskr/src/multisource.rs:1332:        let server = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1358:        let fetch = tokio::spawn(async move {
crates/slskr/src/vpn.rs:213:        .timeout(Duration::from_millis(options.gluetun.timeout))
crates/slskr/src/vpn.rs:340:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:385:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:425:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:460:            let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:485:        let server = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:130:        relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/signalr_ws.rs:158:    let reader_task = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:161:                relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT)
crates/slskr/src/signalr_ws.rs:170:    let mut keepalive = tokio::time::interval(relay_ws::SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/route_dispatch_group_6.rs:2951:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:570:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:658:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:672:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:678:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:707:            tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:848:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:878:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:894:                match timeout(QUIC_DATA_READ_TIMEOUT, connection.accept_inbound_stream()).await {
crates/slskr/src/private_gateway.rs:920:                        match timeout(QUIC_DATA_READ_TIMEOUT, receive.read_to_end()).await {
crates/slskr/src/private_gateway.rs:949:        let (line, line_bytes) = match read_quic_data_command_line_with_timeout(&mut receive).await
crates/slskr/src/private_gateway.rs:972:            let relay_line = match read_quic_data_command_line_with_timeout(&mut receive).await {
crates/slskr/src/private_gateway.rs:1010:                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
crates/slskr/src/private_gateway.rs:1018:            if timeout(DESTINATION_WRITE_TIMEOUT, send.write_all(b"OK\n"))
crates/slskr/src/private_gateway.rs:1033:            match timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await {
crates/slskr/src/private_gateway.rs:1049:        let remaining = match timeout(
crates/slskr/src/private_gateway.rs:1080:                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
crates/slskr/src/private_gateway.rs:1160:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:1367:async fn read_quic_data_command_line_with_timeout(
crates/slskr/src/private_gateway.rs:1370:    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
crates/slskr/src/private_gateway.rs:1379:    timeout(DESTINATION_WRITE_TIMEOUT, async {
crates/slskr/src/private_gateway.rs:1480:        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
crates/slskr/src/private_gateway.rs:1491:        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
crates/slskr/src/private_gateway.rs:1557:                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
crates/slskr/src/private_gateway.rs:1660:        let search = timeout(Duration::from_secs(5), async {
crates/slskr/src/private_gateway.rs:1876:        let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:2097:        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
crates/slskr/src/private_gateway.rs:2125:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2154:        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))
crates/slskr/src/private_gateway.rs:2275:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2547:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2557:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:3017:        let forwarder = tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:3025:        let (size, source) = tokio::time::timeout(
crates/slskr/src/relay_agent.rs:55:    tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:79:    let relay_target = time::timeout(
crates/slskr/src/relay_agent.rs:100:    let mut socket = time::timeout(
crates/slskr/src/relay_agent.rs:114:    let challenge = time::timeout(RELAY_REQUEST_TIMEOUT, wait_for_challenge(&mut socket))
crates/slskr/src/relay_agent.rs:133:    time::timeout(
crates/slskr/src/relay_agent.rs:142:    let share_token = time::timeout(
crates/slskr/src/relay_agent.rs:177:            messages = time::timeout(
crates/slskr/src/relay_agent.rs:262:        .timeout(RELAY_REQUEST_TIMEOUT)
crates/slskr/src/relay_agent.rs:554:    time::timeout(
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
crates/slskr/src/route_dispatch.rs:272:    tokio::spawn(async move {
crates/slskr/src/focused_controller_tests.rs:60:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
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
crates/slskr/src/persistence.rs:1117:            .busy_timeout(Duration::from_secs(30));
crates/slskr/src/config.rs:1101:        let reconnect_delay = validated_runtime_interval(
crates/slskr/src/config.rs:1110:        let ping_interval = validated_runtime_interval(
crates/slskr/src/config.rs:1302:        let peer_response_timeout = validated_runtime_interval(
crates/slskr/src/config.rs:2708:fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
crates/slskr/src/config.rs:7550:        let timeout_connect = parse_timeout(
crates/slskr/src/config.rs:7561:        let timeout_inactivity = parse_timeout(
crates/slskr/src/config.rs:7576:        let timeout_transfer = parse_timeout(
crates/slskr/src/lib.rs:7676:    fn compile_with_timeout(
crates/slskr/src/lib.rs:7694:                .is_match_with_timeout(value, timeout)
crates/slskr/src/lib.rs:7703:fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
crates/slskr/src/lib.rs:7712:    let match_timeout = controller_regex_timeout(target);
crates/slskr/src/lib.rs:7716:            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
crates/slskr/src/lib.rs:15310:        .timeout(Duration::from_secs(10))
crates/slskr/src/lib.rs:15337:    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
crates/slskr/src/lib.rs:15484:        .timeout(Duration::from_secs(20))
crates/slskr/src/lib.rs:15561:        if let Some(metadata) = tokio::time::timeout(
crates/slskr/src/lib.rs:15728:        tokio::spawn(async move {
crates/slskr/src/lib.rs:18493:    tokio::spawn(async move {
crates/slskr/src/lib.rs:18508:    let _ = time::timeout(
crates/slskr/src/lib.rs:18523:    tokio::spawn(async move {
crates/slskr/src/lib.rs:22032:                 tokio::spawn(async move {
crates/slskr/src/lib.rs:25145:            let interests = match time::timeout(
crates/slskr/src/lib.rs:26267:                tokio::spawn(async move {
crates/slskr/src/lib.rs:33727:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:35792:                tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:37503:    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/lib.rs:37870:    tokio::spawn(async move {
crates/slskr/src/lib.rs:37923:            .timeout(Duration::from_secs(100))
crates/slskr/src/lib.rs:40032:    tokio::spawn(async move {
crates/slskr/src/lib.rs:40036:        let mut interval = time::interval(Duration::from_millis(200));
crates/slskr/src/lib.rs:44417:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44438:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44672:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44726:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:45354:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45649:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45688:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45719:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45746:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:46566:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46608:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47368:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47502:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:48120:    tokio::spawn(async move {
crates/slskr/src/lib.rs:48142:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:48322:        .timeout(timeout)
crates/slskr/src/lib.rs:50082:    tokio::spawn(async move {
crates/slskr/src/lib.rs:51756:                tokio::spawn(async move {
crates/slskr/src/lib.rs:54859:    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
crates/slskr/src/lib.rs:54869:    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
crates/slskr/src/lib.rs:55042:    tokio::time::timeout(BRIDGE_READ_TIMEOUT, bridge_read_frame_inner(stream))
crates/slskr/src/lib.rs:55048:async fn bridge_read_frame_with_timeout(
crates/slskr/src/lib.rs:55052:    tokio::time::timeout(timeout_duration, bridge_read_frame_inner(stream))
crates/slskr/src/lib.rs:55088:    tokio::time::timeout(
crates/slskr/src/lib.rs:55233:    tokio::spawn(async move {
crates/slskr/src/lib.rs:55324:        tokio::spawn(async move {
crates/slskr/src/lib.rs:56414:        .timeout(std::time::Duration::from_secs(5))
crates/slskr/src/lib.rs:57151:    let reply = match time::timeout(
crates/slskr/src/lib.rs:57587:            .timeout(solid.timeout)
crates/slskr/src/lib.rs:58010:        tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:72369:    let response = time::timeout(
crates/slskr/src/lib.rs:72417:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/lib.rs:73233:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73240:        tokio::spawn(dht.run());
crates/slskr/src/lib.rs:73298:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73304:                tokio::spawn(async move {
crates/slskr/src/lib.rs:73329:            tokio::spawn(async move {
crates/slskr/src/lib.rs:73336:                    tokio::spawn(async move {
crates/slskr/src/lib.rs:73395:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73505:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73534:                    wishlist_scheduler.set_server_interval(server_interval);
crates/slskr/src/lib.rs:73565:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:73616:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/lib.rs:73619:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/lib.rs:73623:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:73750:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73794:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73972:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73973:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/lib.rs:73982:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73984:        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
crates/slskr/src/lib.rs:74004:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74037:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74039:        let mut interval = time::interval(Duration::from_secs(30 * 60));
crates/slskr/src/lib.rs:74125:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74168:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74197:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74199:        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
crates/slskr/src/lib.rs:74315:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74316:        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
crates/slskr/src/lib.rs:74937:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:74944:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:75094:    tokio::spawn(async move {
crates/slskr/src/lib.rs:75226:                            tokio::spawn(async move {
crates/slskr/src/lib.rs:75291:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:75350:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:75668:            tokio::spawn(async move {
crates/slskr/src/lib.rs:75987:    let stream = time::timeout(
crates/slskr/src/lib.rs:76031:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:76094:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:76143:            received = time::timeout(
crates/slskr/src/lib.rs:76170:                    if time::timeout(
crates/slskr/src/lib.rs:76653:        let remote_token = time::timeout(
crates/slskr/src/lib.rs:76736:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:77330:    let response = time::timeout(
crates/slskr/src/lib.rs:77388:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:77427:    time::timeout(
crates/slskr/src/lib.rs:77440:    time::timeout(
crates/slskr/src/lib.rs:77676:    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
crates/slskr/src/lib.rs:77738:    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
crates/slskr/src/lib.rs:77854:    tokio::task::spawn_blocking(move || create_application_dump_file(&state_dir))
crates/slskr/src/lib.rs:78296:        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
crates/slskr/src/lib.rs:78303:        time::timeout(io_timeout, preview.connection.send_offset(0))
crates/slskr/src/lib.rs:78313:    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
crates/slskr/src/lib.rs:78322:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:78329:            time::timeout(io_timeout, writer.write_all(&chunk))
crates/slskr/src/lib.rs:78336:    time::timeout(io_timeout, writer.flush())
crates/slskr/src/lib.rs:78358:    time::timeout(io_timeout, async {
crates/slskr/src/lib.rs:80625:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/lib.rs:80887:    tokio::spawn(async move {
crates/slskr/src/lib.rs:81641:    tokio::spawn(async move {
crates/slskr/src/lib.rs:82132:    time::timeout(
crates/slskr/src/lib.rs:82381:            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
crates/slskr/src/lib.rs:82401:    let received_token = time::timeout(
crates/slskr/src/lib.rs:82411:    time::timeout(
crates/slskr/src/lib.rs:82420:    time::timeout(
crates/slskr/src/lib.rs:82944:    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
crates/slskr/src/lib.rs:83021:        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
crates/slskr/src/lib.rs:83317:        time::timeout(
crates/slskr/src/lib.rs:83325:    let offset = time::timeout(
crates/slskr/src/lib.rs:83360:        time::timeout(
crates/slskr/src/lib.rs:83776:    let token = time::timeout(
crates/slskr/src/lib.rs:83789:    time::timeout(
crates/slskr/src/lib.rs:83805:        let chunk = time::timeout(
crates/slskr/src/lib.rs:84090:    let stream = time::timeout(settings.timeout_connect, async {
crates/slskr/src/lib.rs:84292:                    Ok(stream) => time::timeout(
crates/slskr/src/lib.rs:84330:    let stream = time::timeout(
crates/slskr/src/lib.rs:84358:    let stream = time::timeout(
crates/slskr/src/lib.rs:84384:    let stream = time::timeout(
crates/slskr/src/lib.rs:84538:    time::timeout(
crates/slskr/src/lib.rs:84545:    let message = time::timeout(
crates/slskr/src/lib.rs:84566:    time::timeout(
crates/slskr/src/lib.rs:84577:    let message = time::timeout(
crates/slskr/src/lib.rs:84596:    let stream = time::timeout(
crates/slskr/src/lib.rs:84604:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:84608:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84624:    let stream = time::timeout(
crates/slskr/src/lib.rs:84632:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:84636:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84717:                let stream = time::timeout(
crates/slskr/src/lib.rs:84729:                time::timeout(
crates/slskr/src/lib.rs:84737:                let stream = time::timeout(
crates/slskr/src/lib.rs:84745:                time::timeout(
crates/slskr/src/lib.rs:84805:    let stream = time::timeout(
crates/slskr/src/lib.rs:84813:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:84817:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84833:    let stream = time::timeout(
crates/slskr/src/lib.rs:84841:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:84845:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84862:    let stream = time::timeout(
crates/slskr/src/lib.rs:84870:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:84874:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84892:    let stream = time::timeout(
crates/slskr/src/lib.rs:84900:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:84904:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84936:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/lib.rs:85245:        let loki_result = time::timeout(
crates/slskr/src/lib.rs:87533:        tokio::spawn(async move {
crates/slskr/src/lib.rs:88044:    let prune_error = tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:88086:    tokio::spawn(async move {
crates/slskr/src/lib.rs:88088:        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
crates/slskr/src/lib.rs:90428:    let snapshot = tokio::task::spawn_blocking(move || build_share_index(&config))
crates/slskr/src/controller_tests.rs:130:    let proxy = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:208:    let proxy = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:457:        let matcher = super::ControllerRegex::compile_with_timeout(expression, true, None)
crates/slskr/src/controller_tests.rs:471:        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", false, None)
crates/slskr/src/controller_tests.rs:474:        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", true, None)
crates/slskr/src/controller_tests.rs:484:    let matcher = super::ControllerRegex::compile_with_timeout(
crates/slskr/src/controller_tests.rs:532:    let peer_task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:590:    tokio::time::timeout(Duration::from_secs(2), peer_task)
crates/slskr/src/controller_tests.rs:605:    let peer_task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:617:        let response = tokio::time::timeout(Duration::from_secs(2), peer.receive())
crates/slskr/src/controller_tests.rs:1472:    let waiter = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:1481:    let wake = tokio::time::timeout(Duration::from_secs(1), waiter)
crates/slskr/src/controller_tests.rs:2694:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:2733:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:2787:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:3539:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:4573:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:4921:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5125:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5238:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5336:async fn spotify_source_requests_enforce_configured_timeout() {
crates/slskr/src/controller_tests.rs:5343:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5375:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5410:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5445:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5470:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5572:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5868:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7412:    let echo = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7487:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:7729:    let received = tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:7802:        tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:8578:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8584:            tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8777:    let server = tokio::spawn(async move { serve_one_stun_response(&socket, mapped).await });
crates/slskr/src/controller_tests.rs:8792:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8811:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:11359:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11388:        let versioned_response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11479:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11536:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11639:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:12937:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:13142:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:20128:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21397:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21473:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21561:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21628:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21735:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21832:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21905:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21939:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22006:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22111:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22228:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22285:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22391:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22446:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26710:    let peer = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26843:    let source = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27030:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));
crates/slskr/src/controller_tests.rs:27093:    let write = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27832:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27967:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:28109:    match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28418:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28421:            tokio::time::timeout(Duration::from_secs(1), super::bridge_read_frame(&mut first))
crates/slskr/src/controller_tests.rs:28459:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28476:    let reconnected = match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28508:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28749:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28782:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28812:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28843:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28977:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29151:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29181:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29204:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29220:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29244:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29276:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29300:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29305:        super::bridge_read_frame_with_timeout(&mut stream, Duration::from_millis(20)).await
crates/slskr/src/controller_tests.rs:31549:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:35055:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:35230:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43213:    let open = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43296:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43456:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44542:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44608:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44738:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44811:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:45815:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50618:        writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50885:        pod_creates.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50920:        message_writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84843:    let token_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84855:    let profile_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:85450:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:97162:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:99347:        let first_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99389:        let second_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99447:        let partial_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:101030:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:103927:    let handler = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:104021:    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
crates/slskr/src/controller_tests.rs:104022:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:105159:    tokio::time::timeout(Duration::from_secs(1), async {
crates/slskr/src/controller_tests.rs:105187:    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
crates/slskr/src/controller_tests.rs:107240:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:111081:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:114261:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:116413:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:118513:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122463:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122469:                tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122681:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122746:    let download = tokio::spawn(super::multisource::execute(
crates/slskr/src/controller_tests.rs:122753:    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/controller_tests.rs:123384:    let version_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:123926:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:136437:        let task = tokio::spawn(async move { serve_json_fixture(&listener, response).await });
crates/slskr/src/controller_tests.rs:142205:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:142843:    let stream_task = tokio::spawn(async move { live_get(stream_state, &stream_path).await });

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
dashboard/src/pages/Monitoring.tsx:121:          target="_blank"
dashboard/src/components/Sidebar.tsx:67:            target="_blank"
dashboard/src/components/Sidebar.tsx:76:            target="_blank"
web/scripts/audit-react-webui.mjs:614:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:615:      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
web/scripts/audit-react-webui.mjs:616:      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
web/scripts/audit-react-webui.mjs:618:        window.localStorage.setItem(
web/scripts/capture-readme-screenshots.mjs:311:  window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/capture-readme-screenshots.mjs:312:  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');
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
web/src/components/System/ExperienceSettings/index.jsx:89:    const stored = JSON.parse(localStorage.getItem(storageKey) || '{}');
web/src/components/System/ExperienceSettings/index.jsx:126:      localStorage.setItem(storageKey, JSON.stringify(form));
web/src/components/System/ExperienceSettings/index.jsx:135:      localStorage.removeItem(storageKey);
web/src/lib/session.js:18:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:31:  setToken(sessionStorage, token);
web/src/components/Shared/Footer.jsx:193:              target="_blank"
web/src/components/Shared/Footer.jsx:219:              target="_blank"
web/src/components/Shared/Footer.jsx:284:                target="_blank"
web/src/components/Shared/Footer.jsx:304:                  target="_blank"
web/src/components/Shared/Footer.jsx:313:                  target="_blank"
web/src/components/Shared/Footer.jsx:325:                  target="_blank"
web/src/components/Shared/Footer.jsx:335:                target="_blank"
web/src/lib/searches.js:72:// Blocked users management (localStorage-based)
web/src/lib/safeOpen.js:22:    const opened = window.open(url, '_blank', 'noopener,noreferrer');
web/src/lib/communityQualitySignals.js:23:    return window.localStorage;
web/src/lib/storage.js:5:    const value = window.localStorage.getItem(key);
web/src/lib/storage.js:16:    window.localStorage.setItem(key, value);
web/src/lib/storage.js:27:    window.localStorage.removeItem(key);
web/src/lib/storage.js:39:      { length: window.localStorage.length },
web/src/lib/storage.js:40:      (_, index) => window.localStorage.key(index),
web/src/lib/storage.js:51:    const value = window.sessionStorage.getItem(key);
web/src/lib/storage.js:62:    window.sessionStorage.setItem(key, value);
web/src/lib/storage.js:82:    window.sessionStorage.removeItem(key);
web/src/components/Browse/Browse.jsx:9:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:27:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:92:  // Save tabs to localStorage whenever they change
web/src/components/Chat/Chat.jsx:39:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:62:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:211:  // Save tabs to localStorage whenever they change
web/src/components/Rooms/Rooms.jsx:41:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:64:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:149:  // Save tabs to localStorage whenever they change
web/src/components/Search/Detail/SearchDetail.jsx:283:  // Sync hasSavedDefault across tabs/searches when localStorage changes

## Suppressed CI and script failures
scripts/check-web-request-body-limit-differential.sh:24:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:25:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:102:      tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:107:  tail -120 "$log" >&2 || true
.github/workflows/release.yml:348:          previous_tag="$(git describe --tags --match 'release-v*' --abbrev=0 "${GITHUB_SHA}^" 2>/dev/null || true)"
scripts/check-web-cors-differential.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:35:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:136:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:148:      tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:153:  tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:359:    tail -120 "$log" >&2 || true
scripts/check-web-cors-differential.sh:363:  wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-cors-differential.sh:366:    tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:28:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:29:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:110:      tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:115:  tail -120 "$log" >&2 || true
scripts/check-web-no-auth-passthrough-differential.sh:298:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-no-auth-passthrough-differential.sh:305:  tail -120 "$log" >&2 || true
.github/workflows/release-publish.yml:273:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:380:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:419:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:462:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:574:        continue-on-error: true
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
scripts/run-council-scan.sh:14:    "$@" >"$tmp" || true
scripts/check-web-auth-disabled-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:51:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:52:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:118:      tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:123:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:298:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/check-web-auth-credentials-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:49:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:50:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:126:      tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:131:  tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:535:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/check-csp-policy.sh:16:    | rg -v 'assert!\(!' || true
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
scripts/run-council-active-bughunt.sh:35:      "$pattern" "$@" || true
scripts/run-council-active-bughunt.sh:78:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/scan-bug-council-candidates.sh:26:    "$pattern" "$@" || true
scripts/scan-bug-council-candidates.sh:73:  'continue-on-error:|allow_failure:|\|\|[[:space:]]+true|set[[:space:]]+\+e' \
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:85:      sort -u || true
scripts/check-local-identity-leaks.sh:106:  latest_tag="$(git tag --sort=-creatordate --list 'build-main-*' | head -n 1 || true)"
scripts/check-local-identity-leaks.sh:108:    latest_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
scripts/check-web-rate-limiting-differential.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:30:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:119:      tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:124:  tail -120 "$log" >&2 || true
scripts/check-rust-format.sh:63:    diff -u -- "$rust_file" "$formatted_file" || true
scripts/probe-natpmp-mapping.sh:33:            "$collision_private_port" tcp 0 >/dev/null 2>&1 || true
scripts/probe-natpmp-mapping.sh:37:            "$private_port" tcp 0 >/dev/null 2>&1 || true
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
scripts/check-remediation-baseline.sh:37:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:40:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
scripts/generate-vpn-soulseek-accounts.sh:65:  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
scripts/generate-vpn-soulseek-accounts.sh:78:  set +e
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
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
scripts/run-container-shutdown-smoke.sh:8:  docker rm -f "$container_name" >/dev/null 2>&1 || true
scripts/run-container-shutdown-smoke.sh:22:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:35:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:41:  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:48:state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
scripts/run-container-shutdown-smoke.sh:51:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:58:  docker logs "$container_name" 2>&1 || true
scripts/run-container-shutdown-smoke.sh:64:  docker logs "$container_name" 2>&1 || true
scripts/run-proton-public-matrix.sh:222:    set +e
scripts/run-proton-public-matrix.sh:302:    set +e
scripts/run-proton-public-matrix.sh:328:                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
scripts/run-proton-public-matrix.sh:334:                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
scripts/run-proton-public-matrix.sh:421:    wait_for_metadata "$listener" "$metadata_probe" || true
scripts/with-process-memory-guard.sh:70:    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
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
scripts/validate-changelog.sh:15:unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
scripts/check-web-audit.sh:28:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-audit.sh:40:    ' <<<"$report" 2>/dev/null || true
scripts/check-web-audit.sh:54:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
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
