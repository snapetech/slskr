# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Protocol-controlled allocations and lengths
crates/slskr/src/cli.rs:1120:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1347:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:2897:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:3209:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:3660:        .read_chunk(5)
crates/slskr/src/mesh_dht.rs:1102:                &vec![b'x'; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr/src/mesh_dht.rs:1132:        let mut output = vec![0; MAX_DHT_VALUE_BYTES - 1];
crates/slskr/src/bloom_filter.rs:39:            bits: vec![0_u8; bit_size.div_ceil(8)],
crates/slskr/src/webhooks.rs:1381:        let mut persisted = vec![invalid; MAX_WEBHOOKS];
crates/slskr/src/content_discovery.rs:238:        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
crates/slskr/src/content_discovery.rs:247:        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
crates/slskr/src/content_discovery.rs:361:        let mut normalized = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:660:        let mut valid = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:675:        let mut candidates = Vec::with_capacity(valid.len());
crates/slskr/src/content_discovery.rs:839:    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
crates/slskr/src/content_discovery.rs:931:    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
crates/slskr/src/content_discovery.rs:960:    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
crates/slskr-web/src/lib.rs:17789:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/lib.rs:17790:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr/src/port_forwarding.rs:293:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/port_forwarding.rs:784:            data: vec![7; TUNNEL_CHUNK_BYTES],
crates/slskr/src/port_forwarding.rs:794:            data: vec![7; TUNNEL_CHUNK_BYTES + 1],
crates/slskr/src/scripts.rs:325:        let data = vec![b'x'; MAX_SCRIPT_OUTPUT_BYTES + 1];
crates/slskr/src/utils.rs:736:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:754:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:1086:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr/src/dotnet_regex.rs:340:    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
crates/slskr/src/dotnet_regex.rs:355:    let mut named_slots = Vec::with_capacity(named.len());
crates/slskr/src/relay_ws.rs:427:    let mut header = Vec::with_capacity(10);
crates/slskr/src/relay_ws.rs:510:    let mut payload = vec![0_u8; length as usize];
crates/slskr/src/relay_ws.rs:565:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/multisource.rs:480:        let mut sources = Vec::with_capacity(request.sources.len());
crates/slskr/src/multisource.rs:522:        let mut source_busy = vec![false; sources.len()];
crates/slskr/src/multisource.rs:526:        let mut results = Vec::with_capacity(chunks.len());
crates/slskr/src/multisource.rs:760:    let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/search_fallback.rs:37:    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
crates/slskr/src/quic_alpn.rs:172:    let mut output = vec![0_u8; length];
crates/slskr/src/quic_alpn.rs:185:    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
crates/slskr/src/events_ws.rs:259:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:377:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:576:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/events_ws.rs:752:        let payload = vec![b'x'; 1024 * 1024];
crates/slskr/src/relay_agent.rs:774:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/relay_agent.rs:977:        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
crates/slskr/src/http_server.rs:453:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:557:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/http_server.rs:944:        let mut buffer = vec![0_u8; 64 * 1024];
crates/slskr/src/http_server.rs:1100:        let body = vec![b'x'; 100 * 1024];
crates/slskr-client/src/quic_data.rs:588:    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, QuicDataError> {
crates/slskr-client/src/quic_data.rs:987:        let received = receive.read_chunk(&mut buffer).await;
crates/slskr-client/src/quic_data.rs:989:            .read_chunk(&mut buffer)
crates/slskr/src/security_controls.rs:1923:        let mut transformed = Vec::with_capacity(transformed_len);
crates/slskr-client/src/mesh_sync.rs:434:        let mut output = Vec::with_capacity(encoded.len());
crates/slskr-client/src/mesh_sync.rs:1053:            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
crates/slskr/src/route_dispatch.rs:82:    let mut normalized = Vec::with_capacity(terms.len());
crates/slskr-client/src/quic_control.rs:41:    let mut encoded = Vec::with_capacity(key_value_len + 5);
crates/slskr/src/relay.rs:425:        let mut shares = Vec::with_capacity(rows.len());
crates/slskr/src/relay.rs:1531:        let mut quotient = Vec::with_capacity(source.len());
crates/slskr/src/relay.rs:1930:        let records = vec![record.clone(); MAX_RELAY_SHARE_UPLOAD_RECORDS + 1];
crates/slskr-client/src/overlay_control.rs:77:        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
crates/slskr-client/src/overlay_control.rs:111:        let payload = reader.read_bytes("payload")?;
crates/slskr-client/src/overlay_control.rs:357:    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
crates/slskr/src/mesh_sync.rs:120:            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
crates/slskr/src/mesh_sync.rs:239:    let mut incoming = Vec::with_capacity(received);
crates/slskr/src/mesh_sync.rs:340:async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
crates/slskr/src/mesh_sync.rs:394:    let mut data = vec![0_u8; to_read];
crates/slskr-client/src/overlay.rs:213:        let mut payload = vec![0_u8; length];
crates/slskr-client/src/overlay.rs:1312:        let mut payload = vec![0; 15];
crates/slskr-client/src/overlay.rs:1543:        let mut signature = vec![0_u8; 64];
crates/slskr-client/src/overlay.rs:1740:                vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr-client/src/overlay.rs:1752:            payload: vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1],
crates/slskr/src/route_dispatch_group_4.rs:1834:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/private_gateway.rs:1207:            let mut response = vec![0_u8; 65_536];
crates/slskr/src/private_gateway.rs:1397:    let mut bytes = Vec::with_capacity(256);
crates/slskr/src/private_gateway.rs:1400:        let read = receive.read_chunk(&mut byte).await?;
crates/slskr/src/private_gateway.rs:1476:            .read_chunk(&mut buffer[..remaining])
crates/slskr/src/private_gateway.rs:1918:            let mut bytes = vec![0_u8; length];
crates/slskr/src/private_gateway.rs:2155:            let mut buffer = vec![0_u8; TUNNEL_CHUNK_BYTES];
crates/slskr/src/private_gateway.rs:3063:        call.payload = vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1];
crates/slskr/src/private_gateway.rs:3103:        let mut packet = vec![0_u8; 1_200];
crates/slskr/src/private_gateway.rs:3347:            vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
crates/slskr/src/route_dispatch_group_2.rs:1825:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr-client/src/search.rs:576:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr-client/src/listener.rs:240:        let mut encoded = Vec::with_capacity(4 + candidate_length);
crates/slskr-client/src/listener.rs:268:    let mut obfuscated = Vec::with_capacity(8 + length);
crates/slskr-client/src/listener.rs:380:            let mut nested = Vec::with_capacity(nested_len);
crates/slskr/src/config.rs:9900:    let mut peers = Vec::with_capacity(values.len());
crates/slskr-client/src/file_transfer.rs:134:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:154:        let mut chunk = vec![0; length];
crates/slskr-client/src/file_transfer.rs:174:        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
crates/slskr-client/src/file_transfer.rs:200:        let mut payload = Vec::with_capacity(length);
crates/slskr-client/src/file_transfer.rs:224:        let mut encoded = Vec::with_capacity(first_block.len() + length);
crates/slskr-client/src/io.rs:215:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:313:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:375:    let mut encoded = Vec::with_capacity(encoded_len);
crates/slskr-client/src/io.rs:407:    let mut obfuscated = Vec::with_capacity(encoded_len);
crates/slskr-client/src/transfer.rs:208:            connection.read_chunk(remaining).await
crates/slskr-client/src/capabilities.rs:173:        let mut features = Vec::with_capacity(feature_count);
crates/slskr-client/src/capabilities.rs:596:    String::from_utf8(reader.read_bytes(length)?.to_vec())
crates/slskr-client/src/capabilities.rs:617:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:668:    let mut output = Vec::with_capacity(values.len());
crates/slskr-protocol/src/primitives.rs:107:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:133:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:134:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:142:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:159:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:192:            output: Vec::with_capacity(capacity),
crates/slskr-protocol/src/peer.rs:727:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-protocol/src/peer.rs:740:        let compressed = compress_zlib(&vec![b'x'; MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES + 1])
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr-protocol/src/distributed.rs:114:                    payload: reader.read_bytes(reader.remaining())?.to_vec(),
crates/slskr-protocol/src/server.rs:1220:                let payload = reader.read_bytes(reader.remaining())?.to_vec();
crates/slskr-protocol/src/server.rs:2106:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2147:    let mut users = Vec::with_capacity(user_count);
crates/slskr-protocol/src/server.rs:2257:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2292:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2347:    let mut values = Vec::with_capacity(count);
crates/slskr-protocol/src/server.rs:2395:    let mut entries = Vec::with_capacity(names.len());
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr/src/lib.rs:6688:            let mut bytes = Vec::with_capacity(33);
crates/slskr/src/lib.rs:10485:        let mut updated = Vec::with_capacity(distinct_ids.len());
crates/slskr/src/lib.rs:14546:    let mut items = Vec::with_capacity(candidates.len());
crates/slskr/src/lib.rs:15768:        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:15770:            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
crates/slskr/src/lib.rs:15772:        "url" => vec!["URL detected; using source query fallback.".to_owned()],
crates/slskr/src/lib.rs:24361:            let mut session_command_permits = Vec::with_capacity(replacements.len());
crates/slskr/src/lib.rs:28798:            let mut visible = Vec::with_capacity(records.len());
crates/slskr/src/lib.rs:37432:    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
crates/slskr/src/lib.rs:47011:        let mut records = Vec::with_capacity(raw_records.len());
crates/slskr/src/lib.rs:49050:    let mut events = Vec::with_capacity(values.len());
crates/slskr/src/lib.rs:49419:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/lib.rs:49818:    let mut requested_files = Vec::with_capacity(files.len());
crates/slskr/src/lib.rs:55337:    let mut payload = vec![0_u8; length - 4];
crates/slskr/src/lib.rs:55441:    let mut provided_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:55442:    let mut configured_padded = vec![0_u8; length];
crates/slskr/src/lib.rs:56540:    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
crates/slskr/src/lib.rs:56650:    let mut lines = Vec::with_capacity(parsed.headers.len());
crates/slskr/src/lib.rs:63133:            let mut results = Vec::with_capacity(work.len());
crates/slskr/src/lib.rs:64585:        let mut current = Vec::with_capacity(right.len() + 1);
crates/slskr/src/lib.rs:65364:        let mut results = Vec::with_capacity(descriptors.len());
crates/slskr/src/lib.rs:65508:        let mut results = Vec::with_capacity(ids.len());
crates/slskr/src/lib.rs:68760:                let mut peers = Vec::with_capacity(peer_records.len());
crates/slskr/src/lib.rs:69292:                let mut entries = Vec::with_capacity(requests.len());
crates/slskr/src/lib.rs:78594:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:82694:        connection.read_chunk(wanted),
crates/slskr/src/lib.rs:83324:    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
crates/slskr/src/lib.rs:83625:    let mut buffer = vec![0_u8; buffer_len];
crates/slskr/src/lib.rs:84084:            connection.read_chunk(next_len),
crates/slskr/src/lib.rs:84236:    let mut order = Vec::with_capacity(2);
crates/slskr/src/lib.rs:84430:            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
crates/slskr/src/lib.rs:84509:    let mut bound_address_and_port = vec![0_u8; address_len + 2];
crates/slskr/src/lib.rs:91624:    let mut actual = vec![0_u8; HEADER.len()];
crates/slskr/src/controller_tests.rs:820:        vec![0; 12]
crates/slskr/src/controller_tests.rs:2747:        let chunk = vec![b' '; 64 * 1024];
crates/slskr/src/controller_tests.rs:2793:                let chunk = vec![b'x'; 64 * 1024];
crates/slskr/src/controller_tests.rs:8739:    let mut attribute = Vec::with_capacity(8);
crates/slskr/src/controller_tests.rs:8745:    let mut response = Vec::with_capacity(32);
crates/slskr/src/controller_tests.rs:19311:        record.results = vec![template.clone(); super::MAX_SEARCH_RESULTS_PER_SEARCH];
crates/slskr/src/controller_tests.rs:21737:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22113:        file.read_chunk(3).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22218:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22304:        file.read_chunk(2).await.expect("chunk")
crates/slskr/src/controller_tests.rs:22469:    assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
crates/slskr/src/controller_tests.rs:24269:        record.members = vec![template.clone(); super::MAX_SHARE_GROUP_MEMBERS];
crates/slskr/src/controller_tests.rs:24431:        record.items = vec![template.clone(); super::MAX_COLLECTION_ITEMS];
crates/slskr/src/controller_tests.rs:28797:        let mut frame = Vec::with_capacity(4 + length as usize);
crates/slskr/src/controller_tests.rs:28912:            let mut actual = vec![0_u8; expected.len()];
crates/slskr/src/controller_tests.rs:103572:        vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103892:        vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
crates/slskr/src/controller_tests.rs:103952:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:103994:    let mut header = vec![0_u8; 42];
crates/slskr/src/controller_tests.rs:104150:            let mut bytes = vec![0_u8; 65_536];
crates/slskr/src/controller_tests.rs:118266:        vec![0_u8; 64 * 1024 + 1],
crates/slskr/src/controller_tests.rs:120206:    let low = entropy.check(&vec![0_u8; EntropyControl::SAMPLE_SIZE]);

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/private_gateway.rs:276:    /// DHT port. DHT-shaped datagrams are forwarded to mainline's internal
crates/slskr/src/private_gateway.rs:806:                            "overlay QUIC proxy closed before initial datagram was forwarded"
crates/slskr/src/private_gateway.rs:3136:        .expect("DHT response should be forwarded")
crates/slskr/src/cli.rs:2499:    let forwarded = tree
crates/slskr/src/cli.rs:2503:    if forwarded != 1 {
crates/slskr/src/cli.rs:2505:            "distributed search reached {forwarded} children instead of one"
crates/slskr/src/webhooks.rs:584:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:585:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:808:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/webhooks.rs:809:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/webhooks.rs:812:            client_builder = client_builder.resolve(&resolved.host, *addr);
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
crates/slskr/src/http_server.rs:67:    pub forwarded: Option<String>,
crates/slskr/src/http_server.rs:68:    pub x_forwarded_for: Option<String>,
crates/slskr/src/http_server.rs:122:                    "forwarded" => headers.forwarded = Some(value.to_string()),
crates/slskr/src/http_server.rs:123:                    "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
crates/slskr/src/http_server.rs:379:            "forwarded" => append_list_header(&mut headers.forwarded, value),
crates/slskr/src/http_server.rs:380:            "x-forwarded-for" => append_list_header(&mut headers.x_forwarded_for, value),
crates/slskr/src/http_server.rs:1062:            headers.forwarded,
crates/slskr/src/http_server.rs:1066:            headers.x_forwarded_for,
crates/slskr/src/http_server.rs:1263:            request.headers.x_forwarded_for.as_deref(),
crates/slskr/src/http_server.rs:1267:            request.headers.forwarded.as_deref(),
crates/slskr/src/relay_agent.rs:261:) -> Result<reqwest::Client, String> {
crates/slskr/src/relay_agent.rs:262:    let mut builder = reqwest::Client::builder()
crates/slskr/src/relay_agent.rs:263:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/relay_agent.rs:739:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:821:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:941:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:1002:    client: &reqwest::Client,
crates/slskr/src/relay_agent.rs:1038:    client: &reqwest::Client,
crates/slskr/src/application_state.rs:43:        "forwardedPort": runtime.vpn.forwarded_port,
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
crates/slskr/src/route_dispatch_group_7.rs:2164:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/route_dispatch_group_7.rs:2368:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:15470:        .to_socket_addrs()
crates/slskr/src/lib.rs:15480:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15482:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:15654:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:15656:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:36856:                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
crates/slskr/src/lib.rs:37063:                Err(error) if error.contains("already being forwarded") => {
crates/slskr/src/lib.rs:38165:        let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:38166:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:42874:                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
crates/slskr/src/lib.rs:44657:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44659:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44678:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44680:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44912:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44914:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:44966:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:44968:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45594:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45596:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45868:        .to_socket_addrs()
crates/slskr/src/lib.rs:45889:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45891:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45928:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45930:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45959:    let response = reqwest::Client::builder()
crates/slskr/src/lib.rs:45961:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:45986:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:45988:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46806:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46808:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46811:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:46848:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:46850:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:46853:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47608:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47610:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47613:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:47742:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:47744:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:47747:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:48382:    let mut builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:48384:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48387:        builder = builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:48562:    let client = reqwest::Client::builder()
crates/slskr/src/lib.rs:48564:        .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:48919:                .to_socket_addrs()
crates/slskr/src/lib.rs:48936:        .to_socket_addrs()
crates/slskr/src/lib.rs:48973:        .to_socket_addrs()
crates/slskr/src/lib.rs:49276:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/lib.rs:49281:fn forwarded_client_ip(
crates/slskr/src/lib.rs:49286:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/lib.rs:49287:        forwarded_header_client_ips(value)?
crates/slskr/src/lib.rs:49289:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/lib.rs:49290:        x_forwarded_for_client_ips(value)?
crates/slskr/src/lib.rs:49293:    forwarded_ips
crates/slskr/src/lib.rs:49305:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:49308:        .map(parse_forwarded_ip_token)
crates/slskr/src/lib.rs:49313:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/lib.rs:49316:        .map(parse_forwarded_element_ip)
crates/slskr/src/lib.rs:49321:fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:49322:    let mut forwarded_ip = None;
crates/slskr/src/lib.rs:49328:        if forwarded_ip.is_some() {
crates/slskr/src/lib.rs:49331:        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
crates/slskr/src/lib.rs:49333:    forwarded_ip
crates/slskr/src/lib.rs:49336:fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/lib.rs:56677:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:56679:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:56681:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/lib.rs:57850:        let mut client_builder = reqwest::Client::builder()
crates/slskr/src/lib.rs:57852:            .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/lib.rs:72635:        reqwest::Client::builder()
crates/slskr/src/lib.rs:72638:            .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/lib.rs:74094:                    "primary" => status.forwarded_port,
crates/slskr/src/lib.rs:74149:/// VPN's forwarded port. The local listener remains bound to the configured
crates/slskr/src/lib.rs:74156:            .forwarded_port
crates/slskr/src/lib.rs:85525:                reqwest::Client::builder()
crates/slskr/src/lib.rs:85528:                    .redirect(reqwest::redirect::Policy::none())
crates/slskr/src/controller_tests.rs:2890:fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
crates/slskr/src/controller_tests.rs:2900:        x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
crates/slskr/src/controller_tests.rs:2905:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3376:fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
crates/slskr/src/controller_tests.rs:3382:        forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
crates/slskr/src/controller_tests.rs:3387:        .expect("trusted forwarded address");
crates/slskr/src/controller_tests.rs:3393:fn forwarded_ip_parser_rejects_malformed_authorities() {
crates/slskr/src/controller_tests.rs:3406:            super::parse_forwarded_ip_token(malformed),
crates/slskr/src/controller_tests.rs:3412:        super::parse_forwarded_ip_token("\"[2001:db8::42]:443\""),
crates/slskr/src/controller_tests.rs:3416:        super::parse_forwarded_ip_token("198.51.100.24:443"),
crates/slskr/src/controller_tests.rs:3423:fn forwarded_elements_require_one_valid_for_parameter() {
crates/slskr/src/controller_tests.rs:3426:        super::parse_forwarded_element_ip("proto=https; for=198.51.100.24; by=10.0.0.2"),
crates/slskr/src/controller_tests.rs:3437:            super::parse_forwarded_element_ip(malformed),
crates/slskr/src/controller_tests.rs:3452:        x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
crates/slskr/src/controller_tests.rs:3457:        .expect("forwarded client address");
crates/slskr/src/controller_tests.rs:3473:        x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
crates/slskr/src/controller_tests.rs:3484:fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
crates/slskr/src/controller_tests.rs:3490:        forwarded: Some("for=unknown".to_owned()),
crates/slskr/src/controller_tests.rs:3491:        x_forwarded_for: Some("203.0.113.99".to_owned()),
crates/slskr/src/controller_tests.rs:6228:        forwarded_port: Some(44_444),
crates/slskr/src/controller_tests.rs:6251:            "forwardedPort": 44444,
crates/slskr/src/controller_tests.rs:93904:            forwarded_port: Some(44_499),
crates/slskr/src/controller_tests.rs:93929:                && application["vpn"]["forwardedPort"] == 44_499
crates/slskr/src/controller_tests.rs:99418:        let client = reqwest::Client::new();

## Filesystem and persistent-state boundaries
crates/slskr/src/virtual_soulfind_v2.rs:583:        std::fs::remove_file(path).expect("remove executable catalogue fixture");
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
crates/slskr/src/realm_subject_index.rs:105:        let mut options = fs::OpenOptions::new();
crates/slskr/src/realm_subject_index.rs:1246:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1262:        fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1275:        fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/realm_subject_index.rs:1284:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/pods.rs:1428:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pods.rs:1571:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1584:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1593:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1599:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1608:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1622:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1662:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1691:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1700:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1725:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1734:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1757:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1766:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1798:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1807:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1832:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1841:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1906:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1915:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1928:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1937:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1962:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pods.rs:1971:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pods.rs:1989:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/relay.rs:1354:        let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1369:        fs::rename(&temporary_path, &manifest_path)
crates/slskr/src/relay.rs:1374:        let _ = fs::remove_file(&temporary_path);
crates/slskr/src/relay.rs:1454:    let mut options = fs::OpenOptions::new();
crates/slskr/src/relay.rs:1782:            tokio::fs::remove_file(path)
crates/slskr/src/relay.rs:1817:        std::fs::create_dir_all(&root).expect("create relay share symlink fixture");
crates/slskr/src/relay.rs:1841:        std::fs::remove_dir_all(root).expect("remove relay share symlink fixture");
crates/slskr/src/relay.rs:1849:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1880:        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
crates/slskr/src/relay.rs:1890:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1912:        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
crates/slskr/src/relay.rs:1922:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1940:        std::fs::remove_dir_all(root).expect("remove oversized manifest fixture");
crates/slskr/src/relay.rs:1950:        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
crates/slskr/src/relay.rs:1967:        std::fs::remove_dir_all(root).expect("remove invalid agent manifest fixture");
crates/slskr/src/relay.rs:2085:        std::fs::create_dir_all(&incoming).expect("create concurrent manifest directory");
crates/slskr/src/relay.rs:2134:        std::fs::remove_dir_all(root).expect("remove concurrent manifest fixture");
crates/slskr/src/multisource.rs:72:            let _ = fs::remove_file(&self.path);
crates/slskr/src/multisource.rs:94:        let _ = fs::remove_dir_all(&self.path);
crates/slskr/src/multisource.rs:474:    fs::create_dir_all(parent).map_err(|_| "output directory could not be created".to_owned())?;
crates/slskr/src/multisource.rs:603:        let _ = fs::remove_file(&assembly_path);
crates/slskr/src/multisource.rs:792:    fs::remove_file(assembly_path)
crates/slskr/src/multisource.rs:821:    let mut options = fs::OpenOptions::new();
crates/slskr/src/multisource.rs:1136:        fs::remove_dir_all(root).expect("remove permissions test root");
crates/slskr/src/multisource.rs:1208:        fs::remove_dir_all(root).expect("remove swarm test root");
crates/slskr/src/multisource.rs:1277:        fs::remove_dir_all(root).expect("remove swarm cancellation test root");
crates/slskr/src/multisource.rs:1306:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/multisource.rs:1367:        fs::remove_dir_all(root).expect("remove mesh preview test root");
crates/slskr/src/scripts.rs:103:    tokio::fs::create_dir_all(script_directory)
crates/slskr/src/scripts.rs:278:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:302:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:320:        tokio::fs::remove_dir_all(directory).await.unwrap();
crates/slskr/src/scripts.rs:388:        tokio::fs::remove_dir_all(directory).await.unwrap();
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
crates/slskr/src/focused_controller_tests.rs:1493:    let _ = fs::remove_dir_all(&state.config.state_dir);
crates/slskr/src/private_gateway.rs:2728:    fs::create_dir_all(state_dir)
crates/slskr/src/private_gateway.rs:2754:        return match fs::remove_file(certificate_path) {
crates/slskr/src/private_gateway.rs:2783:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2826:    let mut options = fs::OpenOptions::new();
crates/slskr/src/private_gateway.rs:2838:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2843:        let _ = fs::remove_file(&temporary);
crates/slskr/src/private_gateway.rs:2846:    if let Err(error) = fs::remove_file(&temporary) {
crates/slskr/src/private_gateway.rs:2864:        fs::create_dir_all(&path).unwrap();
crates/slskr/src/private_gateway.rs:3231:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3257:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3287:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3330:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3339:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3353:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3368:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3379:        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
crates/slskr/src/private_gateway.rs:3384:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3400:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3414:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/private_gateway.rs:3433:        fs::remove_dir_all(root).unwrap();
crates/slskr/src/route_dispatch_group_2.rs:3301:    match tokio::fs::remove_file(path).await {
crates/slskr/src/content_discovery.rs:989:    let mut options = fs::OpenOptions::new();
crates/slskr/src/content_discovery.rs:1367:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1391:        fs::remove_dir_all(root).expect("remove state directory");
crates/slskr/src/content_discovery.rs:1400:        fs::create_dir_all(&root).expect("create state directory");
crates/slskr/src/content_discovery.rs:1419:        fs::remove_dir_all(root).expect("remove state directory");
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
crates/slskr/src/pod_channels.rs:434:    let mut options = fs::OpenOptions::new();
crates/slskr/src/pod_channels.rs:534:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:556:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:565:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:590:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:599:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:624:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:633:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:657:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/pod_channels.rs:666:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/pod_channels.rs:682:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/relay_agent.rs:763:    fs::create_dir_all(&relay_directory)
crates/slskr/src/relay_agent.rs:798:    let cleanup = fs::remove_file(&database_path).await;
crates/slskr/src/relay_agent.rs:1129:    fs::rename(&temporary, &destination)
crates/slskr/src/relay_agent.rs:1138:    let mut options = fs::OpenOptions::new();
crates/slskr/src/relay_agent.rs:1183:            match std::fs::remove_file(&self.path) {
crates/slskr/src/relay_agent.rs:1303:        fs::remove_file(path)
crates/slskr/src/relay_agent.rs:1338:        std::fs::remove_file(path).unwrap();
crates/slskr/src/mesh_security.rs:1310:                fs::create_dir_all(&mesh_directory)
crates/slskr/src/mesh_security.rs:1465:        let mut options = fs::OpenOptions::new();
crates/slskr/src/mesh_security.rs:2150:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/http_server.rs:1771:        std::fs::remove_file(path).unwrap();
crates/slskr/src/http_server.rs:1813:        std::fs::remove_file(path).unwrap();
crates/slskr/src/persistence.rs:21:    let file = OpenOptions::new()
crates/slskr/src/persistence.rs:34:    file.set_permissions(std::fs::Permissions::from_mode(0o600))
crates/slskr/src/persistence.rs:5692:        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();
crates/slskr/src/persistence.rs:5708:        std::fs::remove_dir_all(root).unwrap();
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
crates/slskr/src/lib.rs:3738:    match tokio::fs::remove_file(path).await {
crates/slskr/src/lib.rs:6672:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6698:                file.set_permissions(fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:6706:            let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:6720:            fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:12612:        let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:14301:            types: canonicalize(
crates/slskr/src/lib.rs:14314:            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
crates/slskr/src/lib.rs:14315:            statuses: canonicalize(
crates/slskr/src/lib.rs:15629:    let _ = fs::remove_file(&normalized_path);
crates/slskr/src/lib.rs:16517:        match existing.canonicalize() {
crates/slskr/src/lib.rs:16563:    let writable = fs::OpenOptions::new()
crates/slskr/src/lib.rs:16569:        let _ = fs::remove_file(probe);
crates/slskr/src/lib.rs:17210:            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
crates/slskr/src/lib.rs:17888:            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
crates/slskr/src/lib.rs:18971:        fs::rename(&temporary, &path)
crates/slskr/src/lib.rs:37453:        .canonicalize()
crates/slskr/src/lib.rs:37482:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/lib.rs:37505:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/lib.rs:37611:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:37658:    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:37659:    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
crates/slskr/src/lib.rs:40251:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:43068:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:44544:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:44637:        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
crates/slskr/src/lib.rs:44645:    match fs::remove_file(path) {
crates/slskr/src/lib.rs:47704:    let directory = fs::canonicalize(directory)
crates/slskr/src/lib.rs:47712:        fs::remove_file(&path).map_err(|error| {
crates/slskr/src/lib.rs:51259:                                    let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:51265:                            let _ = fs::remove_file(&database_path);
crates/slskr/src/lib.rs:51281:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:51292:    if let Err(error) = fs::remove_file(path) {
crates/slskr/src/lib.rs:51334:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:51350:        let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:70855:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:70872:            .canonicalize()
crates/slskr/src/lib.rs:70879:                .canonicalize()
crates/slskr/src/lib.rs:70884:                .canonicalize()
crates/slskr/src/lib.rs:72567:        fs::remove_file(path)
crates/slskr/src/lib.rs:72571:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:72576:    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
crates/slskr/src/lib.rs:73695:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/lib.rs:73707:        std::fs::create_dir_all(path)
crates/slskr/src/lib.rs:73754:    std::fs::create_dir_all(path).map_err(|error| {
crates/slskr/src/lib.rs:74952:    let _ = fs::remove_file(output_path);
crates/slskr/src/lib.rs:77819:        let canonical_path = local_path.canonicalize().ok()?;
crates/slskr/src/lib.rs:77823:            .filter_map(|root| root.canonicalize().ok())
crates/slskr/src/lib.rs:77839:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:78133:    fs::create_dir_all(&directory)
crates/slskr/src/lib.rs:78138:        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
crates/slskr/src/lib.rs:78156:    let file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:78164:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:78229:                let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:78241:        let _ = fs::remove_file(&output_path);
crates/slskr/src/lib.rs:78540:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:78547:            let _ = fs::remove_file(&path);
crates/slskr/src/lib.rs:79134:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/lib.rs:79142:            .canonicalize()
crates/slskr/src/lib.rs:79144:        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
crates/slskr/src/lib.rs:79164:            fs::remove_dir_all(&path)
crates/slskr/src/lib.rs:79170:            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/lib.rs:79310:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/lib.rs:79318:        fs::create_dir_all(parent)
crates/slskr/src/lib.rs:79322:        .canonicalize()
crates/slskr/src/lib.rs:79327:        .canonicalize()
crates/slskr/src/lib.rs:79419:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:79470:        .canonicalize()
crates/slskr/src/lib.rs:79473:        .canonicalize()
crates/slskr/src/lib.rs:79478:    fs::OpenOptions::new()
crates/slskr/src/lib.rs:83131:        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
crates/slskr/src/lib.rs:83143:            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
crates/slskr/src/lib.rs:83812:            fs::OpenOptions::new()
crates/slskr/src/lib.rs:83878:        fs::rename(&final_path, &incomplete_path)
crates/slskr/src/lib.rs:83906:        fs::remove_file(&completed_path)
crates/slskr/src/lib.rs:83909:    match fs::rename(&incomplete_path, &completed_path) {
crates/slskr/src/lib.rs:83917:            fs::remove_file(&incomplete_path)
crates/slskr/src/lib.rs:84029:        fs::create_dir_all(&root)
crates/slskr/src/lib.rs:84036:        fs::rename(path, destination)
crates/slskr/src/lib.rs:84039:        fs::remove_file(path)
crates/slskr/src/lib.rs:85482:        match tokio::fs::create_dir_all(&log_dir).await {
crates/slskr/src/lib.rs:85484:                match tokio::fs::OpenOptions::new()
crates/slskr/src/lib.rs:88222:            match fs::remove_dir(&path) {
crates/slskr/src/lib.rs:88254:                match fs::remove_file(&path) {
crates/slskr/src/lib.rs:88489:                    match fs::remove_file(&path) {
crates/slskr/src/lib.rs:89580:                let _ = fs::remove_file(path);
crates/slskr/src/lib.rs:91249:        match root.canonicalize() {
crates/slskr/src/lib.rs:91338:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/lib.rs:91583:    fs::create_dir_all(parent)
crates/slskr/src/lib.rs:91592:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:91648:        fs::remove_file(&rotated_path)
crates/slskr/src/lib.rs:91651:    fs::rename(path, &rotated_path)
crates/slskr/src/lib.rs:91678:    let mut options = fs::OpenOptions::new();
crates/slskr/src/lib.rs:91758:    fs::create_dir_all(parent)?;
crates/slskr/src/lib.rs:91783:        let mut file = fs::OpenOptions::new()
crates/slskr/src/lib.rs:91794:            let _ = fs::remove_file(temp_path);
crates/slskr/src/lib.rs:91812:    fs::rename(source, destination)
crates/slskr/src/lib.rs:91820:    match fs::remove_file(destination) {
crates/slskr/src/lib.rs:91825:    fs::rename(source, destination)
crates/slskr/src/lib.rs:91855:    let mut options = fs::OpenOptions::new();
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
crates/slskr/src/controller_tests.rs:3527:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:4019:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4116:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4390:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4471:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:4648:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:5067:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5236:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5339:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:5983:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6006:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6014:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6106:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6114:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6201:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6270:    fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6334:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:6902:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7314:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7341:    std::fs::create_dir_all(&root).expect("gateway state directory");
crates/slskr/src/controller_tests.rs:7905:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:7913:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:8730:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:9404:    std::fs::create_dir_all(root.join("assets")).unwrap();
crates/slskr/src/controller_tests.rs:9405:    std::fs::create_dir_all(root.join("static")).unwrap();
crates/slskr/src/controller_tests.rs:9440:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9465:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9474:    std::fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:9483:    let _ = std::fs::remove_file(outside);
crates/slskr/src/controller_tests.rs:9484:    let _ = std::fs::remove_dir_all(outside_dir);
crates/slskr/src/controller_tests.rs:9485:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9506:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9550:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:9564:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:9581:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:9824:    std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:9957:    std::fs::create_dir_all(&album).unwrap();
crates/slskr/src/controller_tests.rs:10051:    std::fs::create_dir_all(&dir).unwrap();
crates/slskr/src/controller_tests.rs:10089:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10090:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:10123:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10159:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:10193:        std::fs::create_dir_all(&directory).unwrap();
crates/slskr/src/controller_tests.rs:11678:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:11812:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12214:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12617:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:12724:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13071:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13076:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:13325:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:13330:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14049:    fs::create_dir_all(&evidence_dir).expect("create application evidence directory");
crates/slskr/src/controller_tests.rs:14090:    std::fs::create_dir_all(&root).expect("share root");
crates/slskr/src/controller_tests.rs:14138:    std::fs::remove_dir_all(root).expect("remove share root");
crates/slskr/src/controller_tests.rs:14334:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14516:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:14753:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:16827:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19494:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:19709:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20082:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20299:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20642:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:20724:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:21162:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21261:    std::fs::create_dir_all(parent).expect("download parent dir");
crates/slskr/src/controller_tests.rs:21271:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21290:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21291:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21298:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21317:    std::fs::create_dir_all(&root).expect("download root");
crates/slskr/src/controller_tests.rs:21318:    std::fs::create_dir_all(&outside).expect("outside directory");
crates/slskr/src/controller_tests.rs:21327:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:21345:    std::fs::create_dir_all(&dir).expect("test dir");
crates/slskr/src/controller_tests.rs:21351:    std::fs::remove_file(&shared_path).expect("remove shared file");
crates/slskr/src/controller_tests.rs:21361:    let _ = std::fs::remove_dir_all(dir);
crates/slskr/src/controller_tests.rs:21382:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:21383:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:21394:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:21395:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:21434:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21757:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21777:    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
crates/slskr/src/controller_tests.rs:21863:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:21937:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22028:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22133:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22236:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22327:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:22477:    let _ = std::fs::remove_file(path);
crates/slskr/src/controller_tests.rs:26555:    std::fs::create_dir_all(&root).expect("create stream share root");
crates/slskr/src/controller_tests.rs:26623:    std::fs::remove_dir_all(root).expect("remove stream fixture");
crates/slskr/src/controller_tests.rs:26658:    std::fs::create_dir_all(&root).expect("create preview share root");
crates/slskr/src/controller_tests.rs:26722:    std::fs::remove_dir_all(root).expect("remove preview fixture");
crates/slskr/src/controller_tests.rs:27038:    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
crates/slskr/src/controller_tests.rs:27126:    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");
crates/slskr/src/controller_tests.rs:27129:    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
crates/slskr/src/controller_tests.rs:27130:    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
crates/slskr/src/controller_tests.rs:27131:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:27302:    std::fs::create_dir_all(&child).unwrap();
crates/slskr/src/controller_tests.rs:27328:        std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:27335:        std::fs::remove_dir_all(outside).unwrap();
crates/slskr/src/controller_tests.rs:27338:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:27486:    let _ = std::fs::remove_file(&queue.state_path);
crates/slskr/src/controller_tests.rs:27487:    let _ = std::fs::remove_file(&queue.events_path);
crates/slskr/src/controller_tests.rs:27959:    fs::create_dir_all(&root).expect("create overlay search state directory");
crates/slskr/src/controller_tests.rs:28084:    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
crates/slskr/src/controller_tests.rs:28094:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28215:    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
crates/slskr/src/controller_tests.rs:28462:    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
crates/slskr/src/controller_tests.rs:28468:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:28767:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:29002:    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
crates/slskr/src/controller_tests.rs:29178:    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
crates/slskr/src/controller_tests.rs:29321:    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
crates/slskr/src/controller_tests.rs:29770:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:29946:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:31537:    fs::create_dir_all(&config.downloads_dir).unwrap();
crates/slskr/src/controller_tests.rs:31546:    fs::create_dir_all(&outside_dir).unwrap();
crates/slskr/src/controller_tests.rs:31557:    fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:31601:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:32064:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:32210:    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32487:    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
crates/slskr/src/controller_tests.rs:32548:    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
crates/slskr/src/controller_tests.rs:32549:    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
crates/slskr/src/controller_tests.rs:32884:    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
crates/slskr/src/controller_tests.rs:33138:    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
crates/slskr/src/controller_tests.rs:33378:    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
crates/slskr/src/controller_tests.rs:33390:    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
crates/slskr/src/controller_tests.rs:33493:    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
crates/slskr/src/controller_tests.rs:34379:    std::fs::create_dir_all(&root).expect("create listening-party share root");
crates/slskr/src/controller_tests.rs:34470:    std::fs::remove_dir_all(root).expect("remove listening-party fixture");
crates/slskr/src/controller_tests.rs:35079:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35263:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35394:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35706:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:35858:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:36061:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:36602:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39236:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39317:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:39492:    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
crates/slskr/src/controller_tests.rs:39520:    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
crates/slskr/src/controller_tests.rs:40813:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:40824:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:42131:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:43021:    fs::create_dir_all(root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:43070:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:43234:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:43340:        let _ = fs::remove_file(path);
crates/slskr/src/controller_tests.rs:43343:    let _ = fs::remove_file(source);
crates/slskr/src/controller_tests.rs:43947:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:44246:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45856:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:45960:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:47184:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47332:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47522:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47738:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:47939:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48196:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:48489:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49210:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49522:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:49907:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50116:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50156:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:50221:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:50227:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50560:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:50747:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51121:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:51370:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:51833:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:52889:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53149:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:53295:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:54056:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54345:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:54521:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54695:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54761:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54843:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:54912:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:55187:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55518:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:55985:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56366:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:56472:        fs::remove_file(&pods_path).expect("remove channel create state file");
crates/slskr/src/controller_tests.rs:56498:        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
crates/slskr/src/controller_tests.rs:56585:        fs::remove_file(&pods_path).expect("remove channel update state file");
crates/slskr/src/controller_tests.rs:56618:        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
crates/slskr/src/controller_tests.rs:56706:        fs::remove_file(&pods_path).expect("remove channel delete state file");
crates/slskr/src/controller_tests.rs:56732:        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
crates/slskr/src/controller_tests.rs:56810:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:57000:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57239:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57378:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57569:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57768:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:57864:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58160:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:58702:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59091:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59435:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:59876:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60201:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60468:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60587:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:60730:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61498:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61735:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:61962:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62150:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62243:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62393:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62576:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:62857:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63307:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63453:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:63630:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:63886:    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
crates/slskr/src/controller_tests.rs:64020:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:64444:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:64641:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65015:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65160:    fs::create_dir_all(&evidence_dir).expect("create discovery graph edge evidence directory");
crates/slskr/src/controller_tests.rs:65441:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:65686:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66186:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66561:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:66889:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67471:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:67786:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:68014:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68205:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:68578:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:69014:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:69467:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:70292:    fs::create_dir_all(&evidence_dir).expect("create quarantine-jury evidence directory");
crates/slskr/src/controller_tests.rs:70531:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:71065:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:71670:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:71951:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72577:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:72927:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73368:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73704:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:73815:            fs::remove_file(&path).expect("remove message storage file");
crates/slskr/src/controller_tests.rs:73979:        fs::remove_dir(&messages_path).expect("remove blocked global message path");
crates/slskr/src/controller_tests.rs:74131:        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
crates/slskr/src/controller_tests.rs:74157:        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
crates/slskr/src/controller_tests.rs:74188:        fs::remove_dir(&messages_path).expect("remove blocked search message path");
crates/slskr/src/controller_tests.rs:74239:        fs::remove_dir(&messages_path).expect("remove blocked count message path");
crates/slskr/src/controller_tests.rs:74376:            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
crates/slskr/src/controller_tests.rs:74383:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:74486:            fs::remove_file(&path).expect("remove membership storage file");
crates/slskr/src/controller_tests.rs:74529:        fs::remove_dir(&pods_path).expect("remove blocked membership delete path");
crates/slskr/src/controller_tests.rs:74618:        fs::remove_dir(&pods_path).expect("remove blocked membership projection path");
crates/slskr/src/controller_tests.rs:74637:        fs::remove_dir(&pods_path).expect("remove blocked membership stats path");
crates/slskr/src/controller_tests.rs:74690:        fs::remove_dir(&pods_path).expect("remove blocked membership moderation path");
crates/slskr/src/controller_tests.rs:74785:        fs::remove_dir(&pods_path).expect("remove blocked membership publish path");
crates/slskr/src/controller_tests.rs:74869:        fs::remove_dir(&pods_path).expect("remove blocked membership update path");
crates/slskr/src/controller_tests.rs:74952:        fs::remove_dir(&pods_path).expect("remove blocked membership cleanup path");
crates/slskr/src/controller_tests.rs:74981:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:75049:                fs::remove_file(&path).expect("remove discovery feature state file");
crates/slskr/src/controller_tests.rs:75158:        fs::remove_dir(&feature_path).expect("remove blocked discovery registration path");
crates/slskr/src/controller_tests.rs:75246:        fs::remove_dir(&feature_path).expect("remove blocked discovery update path");
crates/slskr/src/controller_tests.rs:75359:        fs::remove_dir(&feature_path).expect("remove blocked discovery unregister path");
crates/slskr/src/controller_tests.rs:75491:        fs::remove_dir(&feature_path).expect("remove blocked discovery projection path");
crates/slskr/src/controller_tests.rs:75551:        fs::remove_dir(&feature_path).expect("remove blocked discovery refresh path");
crates/slskr/src/controller_tests.rs:75640:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:76460:    fs::create_dir_all(&evidence_dir).expect("create PodJoinLeave evidence directory");
crates/slskr/src/controller_tests.rs:76931:    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
crates/slskr/src/controller_tests.rs:77378:    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
crates/slskr/src/controller_tests.rs:78238:    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
crates/slskr/src/controller_tests.rs:78950:    fs::create_dir_all(&evidence_dir).expect("create MultiSource evidence directory");
crates/slskr/src/controller_tests.rs:79365:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79507:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:79763:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:79978:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80243:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80470:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:80501:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81555:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:81814:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:82631:    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
crates/slskr/src/controller_tests.rs:83375:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83679:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:83939:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84240:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:84445:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84651:                    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:84744:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:84862:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85070:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:85075:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85195:    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:85382:    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");
crates/slskr/src/controller_tests.rs:85387:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85577:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:85921:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86170:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86247:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86345:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86435:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86655:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:86834:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86936:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:86999:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87069:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87111:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87163:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87218:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87541:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:87718:    let _ = fs::remove_file(&validation_path);
crates/slskr/src/controller_tests.rs:87881:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88135:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88267:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:88372:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:88553:    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
crates/slskr/src/controller_tests.rs:88772:    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
crates/slskr/src/controller_tests.rs:88932:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89024:    std::fs::create_dir_all(download_file.parent().unwrap())
crates/slskr/src/controller_tests.rs:89082:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:89230:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89316:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89419:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89538:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:89590:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90113:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90485:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90554:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90601:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90651:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90705:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90809:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90866:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90927:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:90972:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91028:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91085:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91202:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91263:    fs::create_dir_all(&custom_path).expect("create destination fixture");
crates/slskr/src/controller_tests.rs:91320:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91324:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91379:    fs::create_dir_all(&root).expect("create destination edge root");
crates/slskr/src/controller_tests.rs:91613:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:91620:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:91860:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:92379:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93102:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93256:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:93496:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:93765:        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
crates/slskr/src/controller_tests.rs:93820:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:93826:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94056:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94126:        std::fs::create_dir_all(&root).expect("create differential downloads root");
crates/slskr/src/controller_tests.rs:94157:        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
crates/slskr/src/controller_tests.rs:94208:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94675:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94886:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:94989:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95473:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95703:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:95864:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:96522:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:97059:    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
crates/slskr/src/controller_tests.rs:97969:    fs::create_dir_all(existing.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:98198:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:98689:    fs::create_dir_all(&new_root).unwrap();
crates/slskr/src/controller_tests.rs:98690:    fs::create_dir_all(&new_downloads).unwrap();
crates/slskr/src/controller_tests.rs:98691:    fs::create_dir_all(&new_incomplete).unwrap();
crates/slskr/src/controller_tests.rs:99089:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99120:        fs::create_dir_all(download_file.parent().unwrap()).expect("downloads fixture root");
crates/slskr/src/controller_tests.rs:99121:        fs::create_dir_all(incomplete_file.parent().unwrap()).expect("incomplete fixture root");
crates/slskr/src/controller_tests.rs:99254:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99359:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99577:        let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:99583:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99607:        fs::create_dir_all(&root).expect("secure writer root");
crates/slskr/src/controller_tests.rs:99671:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:99677:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99699:    fs::create_dir_all(&root).expect("DHT certificate root");
crates/slskr/src/controller_tests.rs:99732:        fs::create_dir_all(&linked_root).expect("DHT symlink root");
crates/slskr/src/controller_tests.rs:99790:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:99797:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100726:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100751:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100752:    let _ = std::fs::remove_file(&outside);
crates/slskr/src/controller_tests.rs:100777:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:100815:    let _ = std::fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:100934:    std::fs::create_dir_all(&nested).expect("create nested dir");
crates/slskr/src/controller_tests.rs:100951:    std::fs::create_dir_all(&album).expect("create recursive directory");
crates/slskr/src/controller_tests.rs:100960:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100981:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:100982:    std::fs::create_dir_all(&outside).expect("create outside");
crates/slskr/src/controller_tests.rs:100995:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:100996:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:101013:    std::fs::create_dir_all(&root).expect("create root");
crates/slskr/src/controller_tests.rs:101028:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101048:    std::fs::create_dir_all(&directory).expect("create deep directory tree");
crates/slskr/src/controller_tests.rs:101058:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:101755:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101762:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101776:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101782:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101841:    std::fs::create_dir_all(&artist).unwrap();
crates/slskr/src/controller_tests.rs:101843:    std::fs::create_dir_all(root.join(".hidden")).unwrap();
crates/slskr/src/controller_tests.rs:101860:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101868:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:101905:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101915:    std::fs::create_dir_all(&first).unwrap();
crates/slskr/src/controller_tests.rs:101916:    std::fs::create_dir_all(&second).unwrap();
crates/slskr/src/controller_tests.rs:101929:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:101968:    std::fs::create_dir_all(&excluded).unwrap();
crates/slskr/src/controller_tests.rs:101989:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:102013:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:102026:    std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/controller_tests.rs:102047:    std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/controller_tests.rs:102048:    std::fs::create_dir_all(&outside).unwrap();
crates/slskr/src/controller_tests.rs:102062:    let _ = std::fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:102063:    let _ = std::fs::remove_dir_all(outside);
crates/slskr/src/controller_tests.rs:102102:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:102122:    std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/controller_tests.rs:102138:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102420:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:102421:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:102928:    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
crates/slskr/src/controller_tests.rs:103002:    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
crates/slskr/src/controller_tests.rs:103041:    let _ = std::fs::remove_file(&path);
crates/slskr/src/controller_tests.rs:103042:    let mut file = std::fs::OpenOptions::new()
crates/slskr/src/controller_tests.rs:103059:    std::fs::remove_file(path).expect("remove cancelled transfer test file");
crates/slskr/src/controller_tests.rs:103100:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103101:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103139:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103140:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103159:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103160:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103209:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103210:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103273:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103274:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103326:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103327:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103390:    let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/controller_tests.rs:103391:    let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/controller_tests.rs:103405:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103442:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103456:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103523:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103568:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103579:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103594:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103606:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103623:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103690:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103704:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103717:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103731:    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
crates/slskr/src/controller_tests.rs:103840:    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:103847:    let _ = fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103862:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103874:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103888:    std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/controller_tests.rs:103944:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:103977:    std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/controller_tests.rs:103986:    let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/controller_tests.rs:104543:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:104978:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:104983:    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
crates/slskr/src/controller_tests.rs:105154:    std::fs::remove_file(picture).unwrap();
crates/slskr/src/controller_tests.rs:105347:    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
crates/slskr/src/controller_tests.rs:105349:    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
crates/slskr/src/controller_tests.rs:105444:        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
crates/slskr/src/controller_tests.rs:105710:        fs::create_dir_all(&downloads_target).expect("create downloads list target");
crates/slskr/src/controller_tests.rs:105711:        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
crates/slskr/src/controller_tests.rs:105746:        let _ = fs::remove_file(downloads_link);
crates/slskr/src/controller_tests.rs:105747:        let _ = fs::remove_file(incomplete_link);
crates/slskr/src/controller_tests.rs:105748:        let _ = fs::remove_dir_all(downloads_target);
crates/slskr/src/controller_tests.rs:105749:        let _ = fs::remove_dir_all(incomplete_target);
crates/slskr/src/controller_tests.rs:105751:    let _ = fs::remove_file(downloads_conflict_root);
crates/slskr/src/controller_tests.rs:105752:    let _ = fs::remove_file(incomplete_conflict_root);
crates/slskr/src/controller_tests.rs:106005:    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106047:    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
crates/slskr/src/controller_tests.rs:106299:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106570:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106650:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:106984:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:107024:    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
crates/slskr/src/controller_tests.rs:107025:    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
crates/slskr/src/controller_tests.rs:107291:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:107379:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
crates/slskr/src/controller_tests.rs:107418:    fs::remove_file(downloads_root.join("Relay/Agent.txt"))
crates/slskr/src/controller_tests.rs:107552:    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
crates/slskr/src/controller_tests.rs:107595:    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
crates/slskr/src/controller_tests.rs:107596:    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");
crates/slskr/src/controller_tests.rs:107721:    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
crates/slskr/src/controller_tests.rs:107763:    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
crates/slskr/src/controller_tests.rs:107764:    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
crates/slskr/src/controller_tests.rs:107765:    let _ = fs::remove_file(database_source);
crates/slskr/src/controller_tests.rs:107766:    let _ = fs::remove_dir_all(downloads_root);
crates/slskr/src/controller_tests.rs:107771:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:108719:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109043:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109382:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:109851:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110596:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:110831:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111119:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111543:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:111792:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:112700:    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
crates/slskr/src/controller_tests.rs:112958:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113268:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:113796:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114075:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114474:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:114895:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115273:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115484:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:115776:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116205:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116451:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:116725:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117244:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:117464:    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
crates/slskr/src/controller_tests.rs:117513:        fs::create_dir_all(&root).expect("path guard root");
crates/slskr/src/controller_tests.rs:117601:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:117683:    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
crates/slskr/src/controller_tests.rs:117786:    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
crates/slskr/src/controller_tests.rs:117949:    fs::create_dir_all(&evidence_dir).expect("create CSRF security evidence directory");
crates/slskr/src/controller_tests.rs:118078:    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
crates/slskr/src/controller_tests.rs:118092:    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");
crates/slskr/src/controller_tests.rs:118140:    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
crates/slskr/src/controller_tests.rs:118187:    fs::create_dir_all(&root).expect("certificate manager root");
crates/slskr/src/controller_tests.rs:118246:    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
crates/slskr/src/controller_tests.rs:118263:    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
crates/slskr/src/controller_tests.rs:118286:        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
crates/slskr/src/controller_tests.rs:118351:    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
crates/slskr/src/controller_tests.rs:118358:    fs::remove_dir_all(&root).expect("remove certificate manager root");
crates/slskr/src/controller_tests.rs:118526:    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
crates/slskr/src/controller_tests.rs:118672:    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
crates/slskr/src/controller_tests.rs:119039:    fs::create_dir_all(&certificate_root).expect("certificate root");
crates/slskr/src/controller_tests.rs:119068:    fs::create_dir_all(&malformed_root).expect("malformed certificate root");
crates/slskr/src/controller_tests.rs:119097:    let _ = fs::remove_dir_all(&certificate_root);
crates/slskr/src/controller_tests.rs:119098:    let _ = fs::remove_dir_all(&malformed_root);
crates/slskr/src/controller_tests.rs:119103:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:119157:    fs::create_dir_all(&root).expect("content-safety root");
crates/slskr/src/controller_tests.rs:119236:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:119240:    fs::create_dir_all(&evidence_dir).expect("create content-safety evidence directory");
crates/slskr/src/controller_tests.rs:119359:    fs::create_dir_all(&evidence_dir).expect("create Soulseek safety evidence directory");
crates/slskr/src/controller_tests.rs:119483:    fs::create_dir_all(&evidence_dir).expect("create security event sink evidence directory");
crates/slskr/src/controller_tests.rs:120029:    std::fs::create_dir_all(&evidence_dir).expect("create integrity evidence directory");
crates/slskr/src/controller_tests.rs:120708:    std::fs::create_dir_all(&evidence_dir).expect("create runtime-control evidence directory");
crates/slskr/src/controller_tests.rs:120918:    std::fs::create_dir_all(&evidence_dir).expect("create route-security evidence directory");
crates/slskr/src/controller_tests.rs:121317:    let _ = fs::remove_dir_all(&root);
crates/slskr/src/controller_tests.rs:121615:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121794:    fs::create_dir_all(&root).expect("JWT revocation root");
crates/slskr/src/controller_tests.rs:121846:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:121851:    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
crates/slskr/src/controller_tests.rs:121972:    fs::create_dir_all(&evidence_dir).expect("create security controller evidence directory");
crates/slskr/src/controller_tests.rs:122056:    fs::create_dir_all(&evidence_dir).expect("create passthrough security evidence directory");
crates/slskr/src/controller_tests.rs:122111:        fs::create_dir_all(&root).expect("authentication control state root");
crates/slskr/src/controller_tests.rs:122270:        let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122276:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:122324:    fs::create_dir_all(&root).expect("pin file lifecycle root");
crates/slskr/src/controller_tests.rs:122366:        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
crates/slskr/src/controller_tests.rs:122390:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122397:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122416:    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
crates/slskr/src/controller_tests.rs:122463:        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
crates/slskr/src/controller_tests.rs:122487:    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
crates/slskr/src/controller_tests.rs:122494:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:122639:    fs::create_dir_all(&root).expect("create multisource lifecycle root");
crates/slskr/src/controller_tests.rs:122915:    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
crates/slskr/src/controller_tests.rs:122924:    let _ = fs::remove_dir_all(root);
crates/slskr/src/controller_tests.rs:123169:        let _ = fs::remove_file(yaml_failure_root);
crates/slskr/src/controller_tests.rs:123341:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:123806:    let _ = fs::remove_file(conflict_root);
crates/slskr/src/controller_tests.rs:124351:    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
crates/slskr/src/controller_tests.rs:124529:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124599:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124757:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:124812:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:124973:        let _ = std::fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125023:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125266:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125404:    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
crates/slskr/src/controller_tests.rs:125548:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125605:    fs::create_dir_all(&evidence_dir).expect("create SongID persistence evidence directory");
crates/slskr/src/controller_tests.rs:125711:    let _ = fs::remove_file(&restart_path);
crates/slskr/src/controller_tests.rs:125749:    fs::create_dir_all(&evidence_dir).expect("create TrafficStats evidence directory");
crates/slskr/src/controller_tests.rs:126373:    fs::create_dir_all(&evidence_dir).expect("create HashDb controller evidence directory");
crates/slskr/src/controller_tests.rs:126469:            fs::remove_file(&path).expect("remove state file before runtime failure");
crates/slskr/src/controller_tests.rs:127686:    fs::create_dir_all(&evidence_dir).expect("create PodsController evidence directory");
crates/slskr/src/controller_tests.rs:128963:    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
crates/slskr/src/controller_tests.rs:129311:    fs::create_dir_all(&evidence_dir)
crates/slskr/src/controller_tests.rs:130314:    fs::create_dir_all(&evidence_dir).expect("create RoomsController evidence directory");
crates/slskr/src/controller_tests.rs:131051:    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
crates/slskr/src/controller_tests.rs:131124:            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
crates/slskr/src/controller_tests.rs:131141:                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
crates/slskr/src/controller_tests.rs:131143:                fs::remove_file(&path).expect("remove prepared PodCore feature file");
crates/slskr/src/controller_tests.rs:133209:    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
crates/slskr/src/controller_tests.rs:133628:        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
crates/slskr/src/controller_tests.rs:133670:        let _ = fs::remove_dir_all(&state_dir);
crates/slskr/src/controller_tests.rs:133693:    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
crates/slskr/src/controller_tests.rs:134487:    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
crates/slskr/src/controller_tests.rs:135036:    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
crates/slskr/src/controller_tests.rs:135181:    fs::create_dir_all(&item_root).expect("create residual library directory");
crates/slskr/src/controller_tests.rs:135295:    let _ = fs::remove_dir_all(&item_root);
crates/slskr/src/controller_tests.rs:135537:    fs::create_dir_all(&evidence_dir).expect("create Library evidence directory");
crates/slskr/src/controller_tests.rs:136468:    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
crates/slskr/src/controller_tests.rs:137029:        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
crates/slskr/src/controller_tests.rs:137487:    fs::create_dir_all(&evidence_dir).expect("create Integrations evidence directory");
crates/slskr/src/controller_tests.rs:138247:    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
crates/slskr/src/controller_tests.rs:138940:    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
crates/slskr/src/controller_tests.rs:139313:    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
crates/slskr/src/controller_tests.rs:139676:    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
crates/slskr/src/controller_tests.rs:140164:    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
crates/slskr/src/controller_tests.rs:140706:    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
crates/slskr/src/controller_tests.rs:141151:    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
crates/slskr/src/controller_tests.rs:141762:    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
crates/slskr/src/controller_tests.rs:142174:    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
crates/slskr/src/controller_tests.rs:142461:    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
crates/slskr/src/controller_tests.rs:142993:    let _ = fs::remove_dir_all(super::effective_downloads_dir(&controller_state));
crates/slskr/src/controller_tests.rs:142994:    let _ = fs::remove_file(share_source);
crates/slskr/src/controller_tests.rs:142999:    fs::create_dir_all(&evidence_dir).expect("create relay evidence directory");
crates/slskr/src/controller_tests.rs:143746:    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
crates/slskr/src/controller_tests.rs:144431:    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
crates/slskr/src/controller_tests.rs:144546:            fs::create_dir_all(&path).expect("create nominal directory");
crates/slskr/src/controller_tests.rs:144609:            fs::create_dir_all(&path).expect("create mutation directory");
crates/slskr/src/controller_tests.rs:144643:            fs::create_dir_all(&path).expect("create concurrent directory");
crates/slskr/src/controller_tests.rs:144681:            fs::create_dir_all(&root).expect("create file storage root");
crates/slskr/src/controller_tests.rs:144733:            fs::create_dir_all(&root).expect("create concurrent file root");
crates/slskr/src/controller_tests.rs:144781:        fs::create_dir_all(&root).expect("create incomplete mutation root");
crates/slskr/src/controller_tests.rs:144860:            fs::create_dir_all(root.join("Album")).expect("create populated root");
crates/slskr/src/controller_tests.rs:144879:            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
crates/slskr/src/controller_tests.rs:144938:            fs::create_dir_all(&album).expect("create populated detail root");
crates/slskr/src/controller_tests.rs:144964:    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");

## Async task and channel lifecycle boundaries
crates/slskr-client/src/quic_data.rs:116:        Some(match timeout(QUIC_CONNECT_TIMEOUT, incoming).await {
crates/slskr-client/src/quic_data.rs:692:    tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:733:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_data.rs:873:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:920:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:966:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:1018:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_data.rs:1059:        let server = tokio::spawn(async move {
crates/slskr-client/src/transfer.rs:156:        self.receive_file_from_with_timeout(
crates/slskr-client/src/transfer.rs:204:        let result = time::timeout(timeout, async {
crates/slskr-client/src/transfer.rs:451:        self.send_file_to_with_timeout(connection, bytes, DEFAULT_TRANSFER_IO_TIMEOUT)
crates/slskr-client/src/transfer.rs:481:        let result = time::timeout(timeout, async {
crates/slskr-client/src/quic_control.rs:253:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:308:        Some(match timeout(QUIC_CONNECT_TIMEOUT, incoming).await {
crates/slskr-client/src/quic_control.rs:385:    let connection = timeout(QUIC_CONNECT_TIMEOUT, connecting)
crates/slskr-client/src/quic_control.rs:402:    tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:451:        let server = tokio::spawn(async move {
crates/slskr-client/src/quic_control.rs:498:        let server = tokio::spawn(async move {
crates/slskr-client/src/peer_cache.rs:125:        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:129:    pub async fn send_to_with_timeout(
crates/slskr-client/src/peer_cache.rs:146:        match time::timeout(timeout, active.send(message)).await {
crates/slskr-client/src/peer_cache.rs:167:        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
crates/slskr-client/src/peer_cache.rs:171:    pub async fn receive_from_with_timeout(
crates/slskr-client/src/peer_cache.rs:187:        match time::timeout(timeout, active.receive()).await {
crates/slskr-client/src/manager.rs:122:        self.ensure_peer_messages_with_timeout(username, DEFAULT_MANAGER_CONNECT_TIMEOUT)
crates/slskr-client/src/manager.rs:126:    pub async fn ensure_peer_messages_with_timeout(
crates/slskr-client/src/manager.rs:136:        time::timeout(timeout, async {
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
crates/slskr-client/src/overlay.rs:58:    let tcp = timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(endpoint))
crates/slskr-client/src/overlay.rs:75:    let tls = timeout(TLS_HANDSHAKE_TIMEOUT, connector.connect(server_name, tcp))
crates/slskr-client/src/overlay.rs:86:    let mut client = timeout(
crates/slskr-client/src/overlay.rs:789:        self.call_with_timeout(call, SERVICE_CALL_TIMEOUT).await
crates/slskr-client/src/overlay.rs:796:        self.search_with_timeout(request, SERVICE_CALL_TIMEOUT)
crates/slskr-client/src/overlay.rs:800:    pub async fn call_with_timeout(
crates/slskr-client/src/overlay.rs:816:        match timeout(deadline, self.call_inner(call)).await {
crates/slskr-client/src/overlay.rs:870:    pub async fn search_with_timeout(
crates/slskr-client/src/overlay.rs:886:        match timeout(deadline, self.search_inner(request)).await {
crates/slskr-client/src/overlay.rs:1303:        let task = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1327:        let writer = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1331:        let decoded = timeout(
crates/slskr-client/src/overlay.rs:1566:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1647:        assert!(timeout(Duration::from_millis(10), wire.read_u8())
crates/slskr-client/src/overlay.rs:1816:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:1882:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:2046:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:2093:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:2108:        let server = tokio::spawn(async move {
crates/slskr-client/src/overlay.rs:2138:            .call_with_timeout(&call, Duration::from_millis(10))
crates/slskr-client/src/overlay.rs:2147:                .call_with_timeout(&call, Duration::from_secs(1))
crates/slskr-client/src/stream.rs:35:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:42:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/search.rs:81:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:128:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:129:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/search.rs:159:    pub fn set_server_interval(&mut self, seconds: Option<u64>) {
crates/slskr/src/mesh_services.rs:407:    timeout(deadline, operation)
crates/slskr/src/mesh_services.rs:553:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:567:        let fetch = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:654:        let server = tokio::spawn(async move {
crates/slskr/src/mesh_services.rs:668:        let fetch = tokio::spawn(async move {
crates/slskr/src/dht.rs:188:        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
crates/slskr/src/dht.rs:201:                match timeout(
crates/slskr/src/dht.rs:246:        timeout(self.lookup_timeout, async {
crates/slskr/src/scripts.rs:21:fn format_timeout(duration: Duration) -> String {
crates/slskr/src/scripts.rs:93:    run_with_timeout(script, script_directory, target, payload, SCRIPT_TIMEOUT).await
crates/slskr/src/scripts.rs:96:async fn run_with_timeout(
crates/slskr/src/scripts.rs:125:    let output = time::timeout(timeout_duration, async {
crates/slskr/src/scripts.rs:140:            format_timeout(timeout_duration)
crates/slskr/src/scripts.rs:215:        tokio::spawn(async move {
crates/slskr/src/scripts.rs:291:        let error = run_with_timeout(
crates/slskr/src/relay_ws.rs:51:    let handshake = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/relay_ws.rs:106:    let reader_task = tokio::spawn(async move {
crates/slskr/src/relay_ws.rs:108:            let frame = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/relay_ws.rs:116:    let mut keepalive = time::interval(SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_ws.rs:415:    time::timeout(
crates/slskr/src/relay_ws.rs:555:    time::timeout(timeout, read_ws_frame(reader))
crates/slskr/src/relay_ws.rs:576:        let error = time::timeout(
crates/slskr/src/relay_ws.rs:578:            read_ws_frame_with_timeout(&mut reader, Duration::from_millis(10)),
crates/slskr/src/route_dispatch_group_3.rs:751:                tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_1.rs:501:                let response = tokio::time::timeout(
crates/slskr/src/route_dispatch_group_1.rs:1464:                tokio::spawn(async move {
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
crates/slskr/src/route_dispatch.rs:272:    tokio::spawn(async move {
crates/slskr/src/events_ws.rs:122:    let reader_task = tokio::spawn(async move {
crates/slskr/src/events_ws.rs:124:            let frame = read_client_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
crates/slskr/src/events_ws.rs:132:    let mut heartbeat = time::interval(heartbeat_interval);
crates/slskr/src/events_ws.rs:287:    time::timeout(timeout, read_client_frame(reader))
crates/slskr/src/events_ws.rs:356:    write_frame_with_timeout(writer, opcode, payload, WEBSOCKET_WRITE_TIMEOUT).await
crates/slskr/src/events_ws.rs:368:    time::timeout(timeout, write_frame_inner(writer, opcode, payload))
crates/slskr/src/events_ws.rs:525:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:530:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:559:        let message = time::timeout(Duration::from_secs(2), async {
crates/slskr/src/events_ws.rs:681:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:704:        let (event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:728:        let (_event_tx, receiver) = broadcast::channel(1);
crates/slskr/src/events_ws.rs:731:        let error = time::timeout(
crates/slskr/src/events_ws.rs:754:            write_frame_with_timeout(&mut writer, 0x82, &payload, Duration::from_millis(50))
crates/slskr/src/events_ws.rs:763:        let error = time::timeout(
crates/slskr/src/events_ws.rs:765:            read_client_frame_with_timeout(&mut reader, Duration::from_millis(10)),
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
crates/slskr/src/batch.rs:410:    fn test_batch_rejects_invalid_timeout() {
crates/slskr/src/route_dispatch_group_2.rs:2879:            let interests = match time::timeout(
crates/slskr/src/focused_controller_tests.rs:60:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/webhooks.rs:610:                .timeout(timeout)
crates/slskr/src/webhooks.rs:683:            tokio::spawn(async move {
crates/slskr/src/webhooks.rs:822:            .timeout(request_timeout)
crates/slskr/src/webhooks.rs:945:    tokio::time::timeout(timeout, resolution)
crates/slskr/src/webhooks.rs:1073:        let server = tokio::spawn(async move {
crates/slskr/src/webhooks.rs:1105:        let server = tokio::spawn(async move {
crates/slskr/src/dotnet_regex.rs:83:    pub fn is_match_with_timeout(&self, value: &str, timeout: Duration) -> Result<bool, String> {
crates/slskr/src/dotnet_regex.rs:107:        match receiver.recv_timeout(timeout) {
crates/slskr/src/multisource.rs:659:        .timeout(SOURCE_TIMEOUT);
crates/slskr/src/multisource.rs:699:    timeout(deadline, resolution)
crates/slskr/src/multisource.rs:899:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:905:                tokio::spawn(async move {
crates/slskr/src/multisource.rs:955:        let task = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1240:        let download = tokio::spawn(execute(
crates/slskr/src/multisource.rs:1316:        let server = tokio::spawn(async move {
crates/slskr/src/multisource.rs:1342:        let fetch = tokio::spawn(async move {
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
crates/slskr/src/mesh_sync.rs:358:    let result = tokio::task::spawn_blocking(move || read_file_chunk(path, offset, length)).await;
crates/slskr/src/vpn.rs:241:        .timeout(Duration::from_millis(options.gluetun.timeout))
crates/slskr/src/vpn.rs:370:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:415:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:455:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:490:            let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:515:        let server = tokio::spawn(async move {
crates/slskr/src/vpn.rs:554:        let server = tokio::spawn(async move {
crates/slskr/src/route_dispatch_group_7.rs:1345:                tokio::spawn(multisource::execute(
crates/slskr/src/signalr_ws.rs:130:        relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT).await?;
crates/slskr/src/signalr_ws.rs:158:    let reader_task = tokio::spawn(async move {
crates/slskr/src/signalr_ws.rs:161:                relay_ws::read_ws_frame_with_timeout(&mut reader, relay_ws::WEBSOCKET_READ_TIMEOUT)
crates/slskr/src/signalr_ws.rs:170:    let mut keepalive = tokio::time::interval(relay_ws::SIGNALR_KEEPALIVE_INTERVAL);
crates/slskr/src/relay_agent.rs:56:    tokio::spawn(async move {
crates/slskr/src/relay_agent.rs:80:    let relay_target = time::timeout(
crates/slskr/src/relay_agent.rs:101:    let mut socket = time::timeout(
crates/slskr/src/relay_agent.rs:115:    let challenge = time::timeout(RELAY_REQUEST_TIMEOUT, wait_for_challenge(&mut socket))
crates/slskr/src/relay_agent.rs:134:    time::timeout(
crates/slskr/src/relay_agent.rs:143:    let share_token = time::timeout(
crates/slskr/src/relay_agent.rs:179:            messages = time::timeout(
crates/slskr/src/relay_agent.rs:264:        .timeout(RELAY_REQUEST_TIMEOUT)
crates/slskr/src/relay_agent.rs:555:    time::timeout(
crates/slskr/src/http_server.rs:187:    read_http_request_with_timeout(reader, REQUEST_READ_TIMEOUT, body_size_limit).await
crates/slskr/src/http_server.rs:195:    time::timeout(timeout, read_http_request_inner(reader, body_size_limit))
crates/slskr/src/http_server.rs:454:        time::timeout(BODY_READ_TIMEOUT, reader.read_exact(&mut buf))
crates/slskr/src/http_server.rs:666:        let available = time::timeout(timeout, reader.fill_buf())
crates/slskr/src/http_server.rs:706:    write_http_response_with_timeout(
crates/slskr/src/http_server.rs:726:    time::timeout(
crates/slskr/src/http_server.rs:741:    time::timeout(
crates/slskr/src/http_server.rs:892:                time::timeout(RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/http_server.rs:927:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(headers.as_bytes()))
crates/slskr/src/http_server.rs:935:            time::timeout(
crates/slskr/src/http_server.rs:948:            let read = time::timeout(RESPONSE_WRITE_TIMEOUT, file.read(&mut buffer[..wanted]))
crates/slskr/src/http_server.rs:955:            time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(&buffer[..read]))
crates/slskr/src/http_server.rs:962:    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.flush())
crates/slskr/src/http_server.rs:1598:        tokio::spawn(async move {
crates/slskr/src/http_server.rs:1606:        let error = read_http_request_with_timeout(
crates/slskr/src/http_server.rs:1681:        let error = write_http_response_with_timeout(
crates/slskr/src/http_server.rs:1849:        tokio::spawn(async move {
crates/slskr/src/persistence.rs:1117:            .busy_timeout(Duration::from_secs(30));
crates/slskr/src/route_dispatch_group_6.rs:2951:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:574:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:683:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:697:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:703:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:732:            tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:885:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:920:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:936:                match timeout(QUIC_DATA_READ_TIMEOUT, connection.accept_inbound_stream()).await {
crates/slskr/src/private_gateway.rs:962:                        match timeout(QUIC_DATA_READ_TIMEOUT, receive.read_to_end()).await {
crates/slskr/src/private_gateway.rs:995:        let (line, line_bytes) = match read_quic_data_command_line_with_timeout(&mut receive).await
crates/slskr/src/private_gateway.rs:1018:            let relay_line = match read_quic_data_command_line_with_timeout(&mut receive).await {
crates/slskr/src/private_gateway.rs:1056:                match timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination)).await {
crates/slskr/src/private_gateway.rs:1064:            if timeout(DESTINATION_WRITE_TIMEOUT, send.write_all(b"OK\n"))
crates/slskr/src/private_gateway.rs:1079:            match timeout(policy.max_relay_duration.max(Duration::from_secs(1)), relay).await {
crates/slskr/src/private_gateway.rs:1095:        let remaining = match timeout(
crates/slskr/src/private_gateway.rs:1126:                match timeout(OVERLAY_MESSAGE_READ_TIMEOUT, connection.accept_envelope()).await {
crates/slskr/src/private_gateway.rs:1206:        tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:1413:async fn read_quic_data_command_line_with_timeout(
crates/slskr/src/private_gateway.rs:1416:    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
crates/slskr/src/private_gateway.rs:1425:    timeout(DESTINATION_WRITE_TIMEOUT, async {
crates/slskr/src/private_gateway.rs:1525:        let tls = timeout(Duration::from_secs(5), self.acceptor.accept(tcp))
crates/slskr/src/private_gateway.rs:1536:        let hello: MeshHello = timeout(Duration::from_secs(5), framer.read())
crates/slskr/src/private_gateway.rs:1602:                let raw = match timeout(liveness.read_wait(), framer.read_raw()).await {
crates/slskr/src/private_gateway.rs:1705:        let search = timeout(Duration::from_secs(5), async {
crates/slskr/src/private_gateway.rs:1916:        let bytes = tokio::task::spawn_blocking(move || {
crates/slskr/src/private_gateway.rs:2137:        let stream = timeout(DESTINATION_CONNECT_TIMEOUT, TcpStream::connect(destination))
crates/slskr/src/private_gateway.rs:2154:        let reader_task = tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2197:        timeout(DESTINATION_WRITE_TIMEOUT, writer.write_all(&request.data))
crates/slskr/src/private_gateway.rs:2325:            tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2601:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2611:    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
crates/slskr/src/private_gateway.rs:2890:        let reader_task = tokio::spawn(async move {
crates/slskr/src/private_gateway.rs:2906:        let result = timeout(Duration::from_secs(1), reader_task)
crates/slskr/src/private_gateway.rs:3123:        let forwarder = tokio::spawn(forward_dht_responses(
crates/slskr/src/private_gateway.rs:3131:        let (size, source) = tokio::time::timeout(
crates/slskr/src/config.rs:1101:        let reconnect_delay = validated_runtime_interval(
crates/slskr/src/config.rs:1110:        let ping_interval = validated_runtime_interval(
crates/slskr/src/config.rs:1302:        let peer_response_timeout = validated_runtime_interval(
crates/slskr/src/config.rs:2708:fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
crates/slskr/src/config.rs:7550:        let timeout_connect = parse_timeout(
crates/slskr/src/config.rs:7561:        let timeout_inactivity = parse_timeout(
crates/slskr/src/config.rs:7576:        let timeout_transfer = parse_timeout(
crates/slskr/src/lib.rs:7847:    fn compile_with_timeout(
crates/slskr/src/lib.rs:7865:                .is_match_with_timeout(value, timeout)
crates/slskr/src/lib.rs:7874:fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
crates/slskr/src/lib.rs:7883:    let match_timeout = controller_regex_timeout(target);
crates/slskr/src/lib.rs:7887:            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
crates/slskr/src/lib.rs:15481:        .timeout(Duration::from_secs(10))
crates/slskr/src/lib.rs:15508:    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
crates/slskr/src/lib.rs:15655:        .timeout(Duration::from_secs(20))
crates/slskr/src/lib.rs:15732:        if let Some(metadata) = tokio::time::timeout(
crates/slskr/src/lib.rs:15899:        tokio::spawn(async move {
crates/slskr/src/lib.rs:18692:    tokio::spawn(async move {
crates/slskr/src/lib.rs:18707:    let _ = time::timeout(
crates/slskr/src/lib.rs:18722:    tokio::spawn(async move {
crates/slskr/src/lib.rs:22231:                 tokio::spawn(async move {
crates/slskr/src/lib.rs:25356:            let interests = match time::timeout(
crates/slskr/src/lib.rs:26481:                tokio::spawn(async move {
crates/slskr/src/lib.rs:33944:                        tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:36037:                tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:37747:    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
crates/slskr/src/lib.rs:38114:    tokio::spawn(async move {
crates/slskr/src/lib.rs:38167:            .timeout(Duration::from_secs(100))
crates/slskr/src/lib.rs:40273:    tokio::spawn(async move {
crates/slskr/src/lib.rs:40277:        let mut interval = time::interval(Duration::from_millis(200));
crates/slskr/src/lib.rs:44658:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44679:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44913:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:44967:        .timeout(Duration::from_secs(spotify.timeout_seconds))
crates/slskr/src/lib.rs:45595:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45890:        .timeout(Duration::from_secs(timeout_seconds))
crates/slskr/src/lib.rs:45929:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45960:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:45987:        .timeout(Duration::from_secs(30))
crates/slskr/src/lib.rs:46807:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:46849:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47609:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:47743:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:48361:    tokio::spawn(async move {
crates/slskr/src/lib.rs:48383:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/lib.rs:48563:        .timeout(timeout)
crates/slskr/src/lib.rs:50304:    tokio::spawn(async move {
crates/slskr/src/lib.rs:52020:                tokio::spawn(async move {
crates/slskr/src/lib.rs:55123:    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
crates/slskr/src/lib.rs:55133:    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
crates/slskr/src/lib.rs:55306:    tokio::time::timeout(BRIDGE_READ_TIMEOUT, bridge_read_frame_inner(stream))
crates/slskr/src/lib.rs:55312:async fn bridge_read_frame_with_timeout(
crates/slskr/src/lib.rs:55316:    tokio::time::timeout(timeout_duration, bridge_read_frame_inner(stream))
crates/slskr/src/lib.rs:55352:    tokio::time::timeout(
crates/slskr/src/lib.rs:55497:    tokio::spawn(async move {
crates/slskr/src/lib.rs:55588:        tokio::spawn(async move {
crates/slskr/src/lib.rs:56678:        .timeout(std::time::Duration::from_secs(5))
crates/slskr/src/lib.rs:57415:    let reply = match time::timeout(
crates/slskr/src/lib.rs:57851:            .timeout(solid.timeout)
crates/slskr/src/lib.rs:58274:        tokio::spawn(multisource::execute(
crates/slskr/src/lib.rs:72633:    let response = time::timeout(
crates/slskr/src/lib.rs:72689:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/lib.rs:73505:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73512:        tokio::spawn(dht.run());
crates/slskr/src/lib.rs:73570:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73576:                tokio::spawn(async move {
crates/slskr/src/lib.rs:73601:            tokio::spawn(async move {
crates/slskr/src/lib.rs:73608:                    tokio::spawn(async move {
crates/slskr/src/lib.rs:73667:        tokio::spawn(async move {
crates/slskr/src/lib.rs:73777:    tokio::spawn(async move {
crates/slskr/src/lib.rs:73806:                    wishlist_scheduler.set_server_interval(server_interval);
crates/slskr/src/lib.rs:73837:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:73888:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/lib.rs:73891:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/lib.rs:73895:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/lib.rs:74022:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74066:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74244:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74245:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/lib.rs:74254:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74256:        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
crates/slskr/src/lib.rs:74276:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74309:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74311:        let mut interval = time::interval(Duration::from_secs(30 * 60));
crates/slskr/src/lib.rs:74397:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74440:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74469:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74471:        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
crates/slskr/src/lib.rs:74587:    tokio::spawn(async move {
crates/slskr/src/lib.rs:74588:        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
crates/slskr/src/lib.rs:75209:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:75216:    tokio::spawn(run_listener_manager(
crates/slskr/src/lib.rs:75366:    tokio::spawn(async move {
crates/slskr/src/lib.rs:75498:                            tokio::spawn(async move {
crates/slskr/src/lib.rs:75563:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:75622:    let incoming = match time::timeout(
crates/slskr/src/lib.rs:75940:            tokio::spawn(async move {
crates/slskr/src/lib.rs:76259:    let stream = time::timeout(
crates/slskr/src/lib.rs:76303:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:76366:    tokio::spawn(run_distributed_link(
crates/slskr/src/lib.rs:76415:            received = time::timeout(
crates/slskr/src/lib.rs:76442:                    if time::timeout(
crates/slskr/src/lib.rs:76925:        let remote_token = time::timeout(
crates/slskr/src/lib.rs:77008:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:77602:    let response = time::timeout(
crates/slskr/src/lib.rs:77660:            match time::timeout(Duration::from_secs(15), peer.receive()).await {
crates/slskr/src/lib.rs:77699:    time::timeout(
crates/slskr/src/lib.rs:77712:    time::timeout(
crates/slskr/src/lib.rs:77948:    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
crates/slskr/src/lib.rs:78010:    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
crates/slskr/src/lib.rs:78126:    tokio::task::spawn_blocking(move || create_application_dump_file(&state_dir))
crates/slskr/src/lib.rs:78568:        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
crates/slskr/src/lib.rs:78575:        time::timeout(io_timeout, preview.connection.send_offset(0))
crates/slskr/src/lib.rs:78585:    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
crates/slskr/src/lib.rs:78594:            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
crates/slskr/src/lib.rs:78601:            time::timeout(io_timeout, writer.write_all(&chunk))
crates/slskr/src/lib.rs:78608:    time::timeout(io_timeout, writer.flush())
crates/slskr/src/lib.rs:78630:    time::timeout(io_timeout, async {
crates/slskr/src/lib.rs:80897:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/lib.rs:81159:    tokio::spawn(async move {
crates/slskr/src/lib.rs:81913:    tokio::spawn(async move {
crates/slskr/src/lib.rs:82404:    time::timeout(
crates/slskr/src/lib.rs:82653:            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
crates/slskr/src/lib.rs:82673:    let received_token = time::timeout(
crates/slskr/src/lib.rs:82683:    time::timeout(
crates/slskr/src/lib.rs:82692:    time::timeout(
crates/slskr/src/lib.rs:83216:    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
crates/slskr/src/lib.rs:83293:        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
crates/slskr/src/lib.rs:83589:        time::timeout(
crates/slskr/src/lib.rs:83597:    let offset = time::timeout(
crates/slskr/src/lib.rs:83637:        time::timeout(
crates/slskr/src/lib.rs:84053:    let token = time::timeout(
crates/slskr/src/lib.rs:84066:    time::timeout(
crates/slskr/src/lib.rs:84082:        let chunk = time::timeout(
crates/slskr/src/lib.rs:84367:    let stream = time::timeout(settings.timeout_connect, async {
crates/slskr/src/lib.rs:84569:                    Ok(stream) => time::timeout(
crates/slskr/src/lib.rs:84607:    let stream = time::timeout(
crates/slskr/src/lib.rs:84635:    let stream = time::timeout(
crates/slskr/src/lib.rs:84661:    let stream = time::timeout(
crates/slskr/src/lib.rs:84815:    time::timeout(
crates/slskr/src/lib.rs:84822:    let message = time::timeout(
crates/slskr/src/lib.rs:84843:    time::timeout(
crates/slskr/src/lib.rs:84854:    let message = time::timeout(
crates/slskr/src/lib.rs:84873:    let stream = time::timeout(
crates/slskr/src/lib.rs:84881:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:84885:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84901:    let stream = time::timeout(
crates/slskr/src/lib.rs:84909:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/lib.rs:84913:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:84994:                let stream = time::timeout(
crates/slskr/src/lib.rs:85006:                time::timeout(
crates/slskr/src/lib.rs:85014:                let stream = time::timeout(
crates/slskr/src/lib.rs:85022:                time::timeout(
crates/slskr/src/lib.rs:85082:    let stream = time::timeout(
crates/slskr/src/lib.rs:85090:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:85094:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:85110:    let stream = time::timeout(
crates/slskr/src/lib.rs:85118:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:85122:    time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:85139:    let stream = time::timeout(
crates/slskr/src/lib.rs:85147:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:85151:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:85169:    let stream = time::timeout(
crates/slskr/src/lib.rs:85177:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/lib.rs:85181:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/lib.rs:85213:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/lib.rs:85522:        let loki_result = time::timeout(
crates/slskr/src/lib.rs:87818:        tokio::spawn(async move {
crates/slskr/src/lib.rs:88413:    let prune_result = match tokio::task::spawn_blocking(move || {
crates/slskr/src/lib.rs:88517:    tokio::spawn(async move {
crates/slskr/src/lib.rs:88519:        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
crates/slskr/src/lib.rs:90889:    let snapshot = tokio::task::spawn_blocking(move || build_share_index(&config))
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
crates/slskr/src/controller_tests.rs:3547:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:4581:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:4929:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5133:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5246:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5344:async fn spotify_source_requests_enforce_configured_timeout() {
crates/slskr/src/controller_tests.rs:5351:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5383:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5418:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5453:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5478:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5580:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:5876:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7420:    let echo = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:7495:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:7737:    let received = tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:7810:        tokio::time::timeout(std::time::Duration::from_secs(2), async {
crates/slskr/src/controller_tests.rs:8586:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8592:            tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8785:    let server = tokio::spawn(async move { serve_one_stun_response(&socket, mapped).await });
crates/slskr/src/controller_tests.rs:8800:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:8819:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:11367:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11396:        let versioned_response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11487:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11544:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:11647:        let response = tokio::time::timeout(
crates/slskr/src/controller_tests.rs:12945:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:13150:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:17517:    let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:20190:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21459:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21535:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21623:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21690:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21797:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21894:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:21967:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22001:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22068:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22173:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22290:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22347:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22453:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:22508:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26772:    let peer = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:26905:    let source = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27092:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));
crates/slskr/src/controller_tests.rs:27155:    let write = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:27894:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28029:    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
crates/slskr/src/controller_tests.rs:28171:    match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28480:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28483:            tokio::time::timeout(Duration::from_secs(1), super::bridge_read_frame(&mut first))
crates/slskr/src/controller_tests.rs:28521:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28538:    let reconnected = match tokio::time::timeout(
crates/slskr/src/controller_tests.rs:28570:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28811:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28844:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28874:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:28905:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29039:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29213:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29243:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29266:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29282:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29306:        tokio::time::timeout(
crates/slskr/src/controller_tests.rs:29338:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29362:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:29367:        super::bridge_read_frame_with_timeout(&mut stream, Duration::from_millis(20)).await
crates/slskr/src/controller_tests.rs:31611:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:35117:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:35292:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43275:    let open = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43358:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:43518:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44604:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44670:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44800:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:44873:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:45877:    let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50703:        writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:50970:        pod_creates.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:51005:        message_writes.push(tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84928:    let token_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:84940:    let profile_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:85535:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:97247:        let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:99432:        let first_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99474:        let second_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:99532:        let partial_server = tokio::spawn(serve_relay_fixture(
crates/slskr/src/controller_tests.rs:101115:    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/controller_tests.rs:104012:    let handler = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:104106:    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
crates/slskr/src/controller_tests.rs:104107:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:105244:    tokio::time::timeout(Duration::from_secs(1), async {
crates/slskr/src/controller_tests.rs:105272:    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
crates/slskr/src/controller_tests.rs:107325:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:111166:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:114346:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:116498:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:118598:    let server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122548:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122554:                tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122766:        let task = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:122831:    let download = tokio::spawn(super::multisource::execute(
crates/slskr/src/controller_tests.rs:122838:    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
crates/slskr/src/controller_tests.rs:123469:    let version_server = tokio::spawn(async move {
crates/slskr/src/controller_tests.rs:124011:        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
crates/slskr/src/controller_tests.rs:136522:        let task = tokio::spawn(async move { serve_json_fixture(&listener, response).await });
crates/slskr/src/controller_tests.rs:142290:        let task = tokio::spawn(super::handle_http_stream(
crates/slskr/src/controller_tests.rs:142928:    let stream_task = tokio::spawn(async move { live_get(stream_state, &stream_path).await });

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
web/scripts/capture-readme-screenshots.mjs:311:  window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/capture-readme-screenshots.mjs:312:  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');
dashboard/src/components/Sidebar.tsx:67:            target="_blank"
dashboard/src/components/Sidebar.tsx:76:            target="_blank"
web/scripts/audit-react-webui.mjs:614:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:615:      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
web/scripts/audit-react-webui.mjs:616:      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
web/scripts/audit-react-webui.mjs:618:        window.localStorage.setItem(
dashboard/src/pages/Monitoring.tsx:122:          target="_blank"
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
web/src/lib/communityQualitySignals.js:41:    return window.localStorage;
web/src/lib/session.js:18:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:31:  setToken(sessionStorage, token);
web/src/components/Browse/Browse.jsx:15:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:37:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:110:  // Save tabs to localStorage whenever they change
web/src/components/Rooms/Rooms.jsx:48:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:58:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:146:  // Save tabs to localStorage whenever they change
web/src/components/Chat/Chat.jsx:46:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:56:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:209:  // Save tabs to localStorage whenever they change
web/src/components/Shared/Footer.jsx:193:              target="_blank"
web/src/components/Shared/Footer.jsx:219:              target="_blank"
web/src/components/Shared/Footer.jsx:284:                target="_blank"
web/src/components/Shared/Footer.jsx:304:                  target="_blank"
web/src/components/Shared/Footer.jsx:313:                  target="_blank"
web/src/components/Shared/Footer.jsx:325:                  target="_blank"
web/src/components/Shared/Footer.jsx:335:                target="_blank"
web/src/lib/searches.js:77:// Blocked users management (localStorage-based)
web/src/components/Search/Detail/SearchDetail.jsx:283:  // Sync hasSavedDefault across tabs/searches when localStorage changes
web/src/lib/storage.js:5:    const value = window.localStorage.getItem(key);
web/src/lib/storage.js:16:    window.localStorage.setItem(key, value);
web/src/lib/storage.js:27:    window.localStorage.removeItem(key);
web/src/lib/storage.js:39:      { length: window.localStorage.length },
web/src/lib/storage.js:40:      (_, index) => window.localStorage.key(index),
web/src/lib/storage.js:51:    const value = window.sessionStorage.getItem(key);
web/src/lib/storage.js:62:    window.sessionStorage.setItem(key, value);
web/src/lib/storage.js:82:    window.sessionStorage.removeItem(key);
web/src/lib/safeOpen.js:22:    const opened = window.open(url, '_blank', 'noopener,noreferrer');

## Suppressed CI and script failures
scripts/run-council-scan.sh:14:    "$@" >"$tmp" || true
.github/workflows/release.yml:366:          previous_tag="$(git describe --tags --match 'release-v*' --abbrev=0 "${GITHUB_SHA}^" 2>/dev/null || true)"
scripts/check-csp-policy.sh:16:    | rg -v 'assert!\(!' || true
.github/workflows/release-publish.yml:273:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:380:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:419:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:462:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:574:        continue-on-error: true
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/start-proton-listener-soak.sh:21:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:22:sudo wg-quick down "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:23:sudo ip link del "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:24:sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/start-proton-listener-soak.sh:25:sudo ip netns del "$namespace" 2>/dev/null || true
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:85:      sort -u || true
scripts/check-local-identity-leaks.sh:106:  latest_tag="$(git tag --sort=-creatordate --list 'build-main-*' | head -n 1 || true)"
scripts/check-local-identity-leaks.sh:108:    latest_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
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
scripts/with-process-memory-guard.sh:70:    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
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
scripts/check-web-auth-disabled-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:51:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:52:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-disabled-differential.sh:118:      tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:123:  tail -120 "$log" >&2 || true
scripts/check-web-auth-disabled-differential.sh:298:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/validate-changelog.sh:15:unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
scripts/check-web-auth-credentials-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:49:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:50:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-auth-credentials-differential.sh:126:      tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:131:  tail -120 "$log" >&2 || true
scripts/check-web-auth-credentials-differential.sh:535:      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
scripts/check-web-rate-limiting-differential.sh:29:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:30:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-rate-limiting-differential.sh:119:      tail -120 "$log" >&2 || true
scripts/check-web-rate-limiting-differential.sh:124:  tail -120 "$log" >&2 || true
scripts/check-rust-format.sh:63:    diff -u -- "$rust_file" "$formatted_file" || true
scripts/check-web-audit.sh:28:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/check-web-audit.sh:40:    ' <<<"$report" 2>/dev/null || true
scripts/check-web-audit.sh:54:      npm --prefix "$package_dir" audit --json 2>/dev/null || true
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
scripts/generate-vpn-soulseek-accounts.sh:65:  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
scripts/generate-vpn-soulseek-accounts.sh:78:  set +e
scripts/check-web-request-body-limit-differential.sh:24:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:25:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-request-body-limit-differential.sh:102:      tail -120 "$log" >&2 || true
scripts/check-web-request-body-limit-differential.sh:107:  tail -120 "$log" >&2 || true
scripts/check-remediation-baseline.sh:37:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
scripts/check-remediation-baseline.sh:40:    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
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
scripts/check-diagnostics-memory-dump-differential.sh:28:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:31:        wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:37:    kill -KILL "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:38:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:116:      tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:121:  tail -120 "$log" >&2 || true
scripts/check-diagnostics-memory-dump-differential.sh:301:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-diagnostics-memory-dump-differential.sh:308:  tail -120 "$log" >&2 || true
scripts/run-slskd-api-compat-smoke.sh:36:    kill "$daemon_pid" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:37:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:22:    kill "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:23:    wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:89:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:105:        unset SLSKD_ENFORCE_SECURITY || true
scripts/check-web-enforce-security-differential.sh:125:      tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:130:  tail -120 "$log" >&2 || true
scripts/check-web-enforce-security-differential.sh:139:      wait "$daemon_pid" 2>/dev/null || true
scripts/check-web-enforce-security-differential.sh:146:  tail -120 "$log" >&2 || true
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
scripts/run-live-soak-proton-natpmp.sh:65:        renew_ports_once || true
scripts/run-live-soak-proton-natpmp.sh:75:        kill "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:76:        wait "$renew_pid" 2>/dev/null || true
scripts/run-live-soak-proton-natpmp.sh:80:            >/dev/null 2>&1 || true
scripts/run-live-soak-proton-natpmp.sh:84:            >/dev/null 2>&1 || true
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
