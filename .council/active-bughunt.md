# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Protocol-controlled allocations and lengths
crates/slskr-protocol/src/obfuscation.rs:6:    let mut output = Vec::with_capacity(4 + input.len());
crates/slskr-client/src/search.rs:364:        let mut drained = Vec::with_capacity(expired.len());
crates/slskr-protocol/src/peer.rs:661:        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
crates/slskr-client/src/capabilities.rs:371:    let mut values = Vec::with_capacity(count);
crates/slskr-client/src/capabilities.rs:382:    let bytes = reader.read_bytes(N)?;
crates/slskr-client/src/capabilities.rs:407:    let mut output = Vec::with_capacity(values.len());
crates/slskr-client/src/io.rs:138:    let mut encoded = Vec::with_capacity(4 + length);
crates/slskr-client/src/io.rs:208:    let mut payload = vec![0; length];
crates/slskr-client/src/io.rs:234:    let mut encoded = Vec::with_capacity(4 + length);
crates/slskr-client/src/io.rs:264:    let mut obfuscated = Vec::with_capacity(8 + length);
crates/slskr-client/src/file_transfer.rs:53:    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
crates/slskr-client/src/file_transfer.rs:69:        let mut chunk = vec![0; length];
crates/slskr-protocol/src/server.rs:1404:    let counts_len = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/server.rs:1413:    let mut entries = Vec::with_capacity(names.len());
crates/slskr-client/src/transfer.rs:125:        let bytes = connection.read_chunk(remaining).await?;
crates/slskr-protocol/src/frame.rs:23:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:38:        let payload = reader.read_bytes(length - 4)?.to_vec();
crates/slskr-protocol/src/frame.rs:77:        let length = reader.read_u32_le()? as usize;
crates/slskr-protocol/src/frame.rs:92:        let payload = reader.read_bytes(length - 1)?.to_vec();
crates/slskr/src/events_ws.rs:187:    let mut payload = vec![0_u8; len as usize];
crates/slskr/src/events_ws.rs:239:    let mut header = Vec::with_capacity(10);
crates/slskr/src/events_ws.rs:350:        let mut frame = Vec::with_capacity(6 + payload.len());
crates/slskr/src/utils.rs:192:    let mut decoded = Vec::with_capacity(bytes.len());
crates/slskr/src/utils.rs:490:    let mut output = Vec::with_capacity(bytes.len());
crates/slskr-protocol/src/primitives.rs:77:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:96:        let length = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:97:        Ok(self.read_bytes(length)?.to_vec())
crates/slskr-protocol/src/primitives.rs:105:        let count = self.read_u32_le()? as usize;
crates/slskr-protocol/src/primitives.rs:122:    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
crates/slskr-protocol/src/primitives.rs:155:            output: Vec::with_capacity(capacity),
crates/slskr/src/http_server.rs:277:        let mut buf = vec![0_u8; content_length];
crates/slskr/src/http_server.rs:548:        let body = vec![b'x'; 100 * 1024];
crates/slskr/src/cli.rs:818:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1040:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1932:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:2220:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:2654:        .read_chunk(5)
crates/slskr-web/src/lib.rs:14019:        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
crates/slskr-web/src/lib.rs:14020:        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
crates/slskr/src/main.rs:20212:    let mut buffer = vec![0_u8; TRANSFER_PROGRESS_CHUNK_BYTES];
crates/slskr/src/main.rs:20313:            connection.read_chunk(next_len),
crates/slskr/src/main.rs:24121:            let chunk = vec![b' '; 64 * 1024];
crates/slskr/src/main.rs:29787:            file.read_chunk(3).await.expect("chunk")
crates/slskr/src/main.rs:30044:            file.read_chunk(3).await.expect("chunk")
crates/slskr/src/main.rs:30143:            file.read_chunk(2).await.expect("chunk")
crates/slskr/src/main.rs:30226:            file.read_chunk(2).await.expect("chunk")
crates/slskr/src/main.rs:30379:        assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
crates/slskr/src/main.rs:34239:            vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
crates/slskr/src/main.rs:34263:            vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],

## Proxy, redirect, SSRF, and outbound trust boundaries
crates/slskr/src/webhooks.rs:426:            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
crates/slskr/src/webhooks.rs:428:            client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/webhooks.rs:508:    let addrs = (host, port).to_socket_addrs()?.collect::<Vec<_>>();
crates/slskr/src/http_server.rs:46:    pub forwarded: Option<String>,
crates/slskr/src/http_server.rs:47:    pub x_forwarded_for: Option<String>,
crates/slskr/src/http_server.rs:86:                    "forwarded" => headers.forwarded = Some(value.to_string()),
crates/slskr/src/http_server.rs:87:                    "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
crates/slskr/src/http_server.rs:244:            "forwarded" => headers.forwarded = Some(value.to_string()),
crates/slskr/src/http_server.rs:245:            "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
crates/slskr/src/http_server.rs:510:            headers.forwarded,
crates/slskr/src/http_server.rs:514:            headers.x_forwarded_for,
crates/slskr/src/main.rs:15539:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/main.rs:15541:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/main.rs:15543:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/main.rs:15576:    let mut client_builder = reqwest::Client::builder()
crates/slskr/src/main.rs:15578:        .redirect(reqwest::redirect::Policy::none());
crates/slskr/src/main.rs:15580:        client_builder = client_builder.resolve(&resolved.host, *addr);
crates/slskr/src/main.rs:15654:                .to_socket_addrs()
crates/slskr/src/main.rs:15671:        .to_socket_addrs()
crates/slskr/src/main.rs:15796:    forwarded_client_ip(config, remote_addr.ip(), headers)
crates/slskr/src/main.rs:15801:fn forwarded_client_ip(
crates/slskr/src/main.rs:15806:    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
crates/slskr/src/main.rs:15807:        forwarded_header_client_ips(value)?
crates/slskr/src/main.rs:15809:        let value = headers.x_forwarded_for.as_deref()?;
crates/slskr/src/main.rs:15810:        x_forwarded_for_client_ips(value)?
crates/slskr/src/main.rs:15813:    forwarded_ips
crates/slskr/src/main.rs:15825:fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/main.rs:15828:        .map(parse_forwarded_ip_token)
crates/slskr/src/main.rs:15833:fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
crates/slskr/src/main.rs:15842:                    .and_then(parse_forwarded_ip_token)
crates/slskr/src/main.rs:15849:fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
crates/slskr/src/main.rs:24150:    fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
crates/slskr/src/main.rs:24160:            x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
crates/slskr/src/main.rs:24165:            .expect("trusted forwarded address");
crates/slskr/src/main.rs:24177:    fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
crates/slskr/src/main.rs:24183:            forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
crates/slskr/src/main.rs:24188:            .expect("trusted forwarded address");
crates/slskr/src/main.rs:24199:            x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
crates/slskr/src/main.rs:24204:            .expect("forwarded client address");
crates/slskr/src/main.rs:24219:            x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
crates/slskr/src/main.rs:24229:    fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
crates/slskr/src/main.rs:24235:            forwarded: Some("for=unknown".to_owned()),
crates/slskr/src/main.rs:24236:            x_forwarded_for: Some("203.0.113.99".to_owned()),

## Filesystem and persistent-state boundaries
crates/slskr/src/credential_store.rs:196:        fs::create_dir_all(parent).map_err(|error| {
crates/slskr/src/credential_store.rs:221:    let mut options = OpenOptions::new();
crates/slskr/src/credential_store.rs:238:    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
crates/slskr/src/config.rs:1175:        let _ = std::fs::remove_file(path);
crates/slskr/src/config.rs:1193:        let _ = std::fs::remove_dir(path);
crates/slskr/src/config.rs:1211:        let _ = std::fs::remove_file(path);
crates/slskr/src/storage.rs:106:    OpenOptions::new()
crates/slskr/src/main.rs:14946:    let canonical_root = root.canonicalize().ok()?;
crates/slskr/src/main.rs:14969:    let canonical_file = file.canonicalize().ok()?;
crates/slskr/src/main.rs:16868:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/main.rs:16876:        .canonicalize()
crates/slskr/src/main.rs:16883:            .canonicalize()
crates/slskr/src/main.rs:16888:            .canonicalize()
crates/slskr/src/main.rs:17899:        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
crates/slskr/src/main.rs:17911:        std::fs::create_dir_all(path)
crates/slskr/src/main.rs:18722:        let canonical_path = local_path.canonicalize().ok()?;
crates/slskr/src/main.rs:18728:            .filter_map(|root| root.canonicalize().ok())
crates/slskr/src/main.rs:18818:    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
crates/slskr/src/main.rs:18820:        .canonicalize()
crates/slskr/src/main.rs:18825:        .canonicalize()
crates/slskr/src/main.rs:18840:        fs::remove_dir_all(&path).map_err(|error| format!("directory delete failed: {error}"))?;
crates/slskr/src/main.rs:18845:        fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
crates/slskr/src/main.rs:18888:        fs::create_dir_all(parent)
crates/slskr/src/main.rs:18891:    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
crates/slskr/src/main.rs:18893:        .canonicalize()
crates/slskr/src/main.rs:18898:        .canonicalize()
crates/slskr/src/main.rs:20266:    let mut file = fs::OpenOptions::new()
crates/slskr/src/main.rs:23369:        match root.canonicalize() {
crates/slskr/src/main.rs:23442:                let Ok(canonical_path) = path.canonicalize() else {
crates/slskr/src/main.rs:23566:        fs::remove_file(&rotated_path)
crates/slskr/src/main.rs:23569:    fs::rename(path, &rotated_path)
crates/slskr/src/main.rs:23650:    match fs::rename(&temp_path, path) {
crates/slskr/src/main.rs:23653:            let _ = fs::remove_file(&temp_path);
crates/slskr/src/main.rs:23664:    let mut file = fs::OpenOptions::new()
crates/slskr/src/main.rs:24058:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/main.rs:24272:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/main.rs:24872:        std::fs::create_dir_all(root.join("assets")).unwrap();
crates/slskr/src/main.rs:24900:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/main.rs:24924:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/main.rs:24931:        let _ = std::fs::remove_file(outside);
crates/slskr/src/main.rs:24932:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/main.rs:24952:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:24978:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:24991:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/main.rs:25008:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/main.rs:25205:        std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
crates/slskr/src/main.rs:25266:        std::fs::create_dir_all(&album).unwrap();
crates/slskr/src/main.rs:25309:        std::fs::create_dir_all(&dir).unwrap();
crates/slskr/src/main.rs:25330:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/main.rs:25357:        std::fs::create_dir_all(&root).unwrap();
crates/slskr/src/main.rs:29523:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:29563:        std::fs::create_dir_all(parent).expect("download parent dir");
crates/slskr/src/main.rs:29572:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/main.rs:29589:        std::fs::create_dir_all(&dir).expect("test dir");
crates/slskr/src/main.rs:29595:        std::fs::remove_file(&shared_path).expect("remove shared file");
crates/slskr/src/main.rs:29602:        let _ = std::fs::remove_dir_all(dir);
crates/slskr/src/main.rs:29640:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:29807:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:29815:        std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
crates/slskr/src/main.rs:29902:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:29974:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:30064:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:30161:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:30249:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:30387:        let _ = std::fs::remove_file(path);
crates/slskr/src/main.rs:33751:        std::fs::create_dir_all(&nested).expect("create nested dir");
crates/slskr/src/main.rs:33767:        let _ = std::fs::remove_dir_all(root);
crates/slskr/src/main.rs:34095:        std::fs::create_dir_all(&artist).unwrap();
crates/slskr/src/main.rs:34097:        std::fs::create_dir_all(root.join(".hidden")).unwrap();
crates/slskr/src/main.rs:34113:        std::fs::remove_dir_all(root).unwrap();
crates/slskr/src/main.rs:34128:        std::fs::create_dir_all(&state_dir).unwrap();
crates/slskr/src/main.rs:34148:        std::fs::remove_dir_all(state_dir).unwrap();
crates/slskr/src/main.rs:34163:        let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/main.rs:34181:        let _ = std::fs::remove_file(queue.events_path);
crates/slskr/src/main.rs:34182:        let _ = std::fs::remove_file(queue.state_path);
crates/slskr/src/main.rs:34195:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/main.rs:34222:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/main.rs:34235:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/main.rs:34246:        let _ = std::fs::remove_dir_all(state_dir);
crates/slskr/src/main.rs:34259:        std::fs::create_dir_all(&state_dir).expect("state dir");
crates/slskr/src/main.rs:34295:        let _ = std::fs::remove_dir_all(state_dir);

## Async task and channel lifecycle boundaries
crates/slskr/src/events_ws.rs:73:    let reader_task = tokio::spawn(async move {
crates/slskr/src/events_ws.rs:83:    let mut heartbeat = time::interval(HEARTBEAT_INTERVAL);
crates/slskr/src/events_ws.rs:304:        let (event_tx, _) = broadcast::channel(10);
crates/slskr/src/events_ws.rs:309:        tokio::spawn(async move {
crates/slskr/src/events_ws.rs:338:        let message = time::timeout(Duration::from_secs(2), socket.next())
crates/slskr/src/webhooks.rs:391:            tokio::spawn(async move {
crates/slskr/src/webhooks.rs:439:            .timeout(std::time::Duration::from_secs(timeout as u64))
crates/slskr/src/webhooks.rs:662:        tokio::spawn(async {
crates/slskr/src/http_server.rs:278:        time::timeout(BODY_READ_TIMEOUT, reader.read_exact(&mut buf))
crates/slskr/src/http_server.rs:344:        let available = time::timeout(timeout, reader.fill_buf())
crates/slskr/src/http_server.rs:794:        tokio::spawn(async move {
crates/slskr-client/src/search.rs:60:    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
crates/slskr-client/src/search.rs:120:    pub fn interval(&self) -> Duration {
crates/slskr-client/src/search.rs:121:        self.options.next_interval(self.server_interval)
crates/slskr-client/src/peer_connect.rs:214:    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:225:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/peer_connect.rs:241:    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:252:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/peer_connect.rs:268:    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/peer_connect.rs:279:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr-client/src/stream.rs:34:        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
crates/slskr-client/src/stream.rs:41:        let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr/src/cli.rs:303:    let stream = time::timeout(
crates/slskr/src/cli.rs:339:            let stream = time::timeout(
crates/slskr/src/cli.rs:397:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:403:        time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:473:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:494:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:508:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:523:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:530:        let response = time::timeout(timeout, plain.receive())
crates/slskr/src/cli.rs:581:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:595:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:634:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:756:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:806:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:818:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:874:            let _ = time::timeout(Duration::from_millis(750), peer.receive()).await;
crates/slskr/src/cli.rs:885:        let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1024:        let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1040:    let bytes = time::timeout(timeout, file.read_chunk(remaining))
crates/slskr/src/cli.rs:1275:            let stream = time::timeout(
crates/slskr/src/cli.rs:1448:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:1488:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:1502:    let echoed = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli.rs:1831:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:1906:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:1920:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:1932:    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
crates/slskr/src/cli.rs:1969:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2015:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:2055:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2086:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:2121:    let server_task = tokio::spawn(async move {
crates/slskr/src/cli.rs:2194:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/cli.rs:2208:    let got_token = time::timeout(timeout, file.receive_token())
crates/slskr/src/cli.rs:2220:    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
crates/slskr/src/cli.rs:2399:        tokio::spawn(async move { run_listener(listener, listener_duration).await });
crates/slskr/src/cli.rs:2402:        tokio::spawn(async move { run_obfuscated_listener(listener, duration).await })
crates/slskr/src/cli.rs:2405:    let watchdog_task = tokio::spawn(run_live_soak_server_watchdog(
crates/slskr/src/cli.rs:2468:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:2538:    let accept_task = tokio::spawn(async move { listener.accept_obfuscated().await });
crates/slskr/src/cli.rs:2604:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:2706:    let accept_task = tokio::spawn(async move { listener.accept().await });
crates/slskr/src/cli.rs:2707:    let stream = time::timeout(timeout, TcpStream::connect(connect_address.as_str()))
crates/slskr/src/cli.rs:2716:    let (incoming, _) = time::timeout(timeout, accept_task)
crates/slskr/src/cli.rs:2774:        match time::timeout(
crates/slskr/src/cli.rs:2816:        match time::timeout(
crates/slskr/src/cli.rs:2918:        match time::timeout(
crates/slskr/src/cli.rs:2957:        match time::timeout(
crates/slskr/src/cli.rs:3013:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:3029:    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
crates/slskr/src/cli.rs:3077:        match time::timeout(
crates/slskr/src/cli.rs:3195:            time::timeout(send_timeout, session.send_ping())
crates/slskr/src/cli.rs:3206:                time::timeout(
crates/slskr/src/cli.rs:3226:        match time::timeout(next_wait, session.receive()).await {
crates/slskr/src/cli.rs:3334:            match time::timeout(timeout, handle_live_soak_connect_to_peer_response(response)).await
crates/slskr/src/cli.rs:3404:    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
crates/slskr/src/cli.rs:3417:        let peer_response = match time::timeout(timeout, peer.receive()).await {
crates/slskr/src/cli.rs:3456:        let token = time::timeout(timeout, transfer.receive_token())
crates/slskr/src/cli.rs:3481:        match time::timeout(
crates/slskr/src/cli.rs:3512:        match time::timeout(
crates/slskr/src/cli.rs:3546:            match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli.rs:3597:            let message = time::timeout(Duration::from_secs(5), distributed.receive())
crates/slskr/src/cli.rs:3614:            let token = time::timeout(Duration::from_secs(5), transfer.receive_token())
crates/slskr/src/cli.rs:3633:        match time::timeout(Duration::from_secs(5), peer.receive()).await {
crates/slskr/src/cli.rs:3679:    match time::timeout(Duration::from_secs(5), peer.receive_user_info_request()).await {
crates/slskr/src/main.rs:8210:                 tokio::spawn(async move {
crates/slskr/src/main.rs:15540:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/main.rs:15577:        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
crates/slskr/src/main.rs:17484:    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
crates/slskr/src/main.rs:17865:        tokio::spawn(async move {
crates/slskr/src/main.rs:17922:    tokio::spawn(async move {
crates/slskr/src/main.rs:17931:        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/main.rs:17955:                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
crates/slskr/src/main.rs:17958:                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
crates/slskr/src/main.rs:17962:                                    Instant::now() + wishlist_scheduler.interval();
crates/slskr/src/main.rs:18022:    tokio::spawn(async move {
crates/slskr/src/main.rs:18023:        let mut interval = time::interval(Duration::from_secs(60));
crates/slskr/src/main.rs:18033:        tokio::spawn(run_listener(Arc::clone(&state), bind, false));
crates/slskr/src/main.rs:18036:        tokio::spawn(run_listener(Arc::clone(&state), bind, true));
crates/slskr/src/main.rs:18069:            time::timeout(
crates/slskr/src/main.rs:18075:            time::timeout(state.config.peer_response_timeout, listener.accept()).await
crates/slskr/src/main.rs:18107:                tokio::spawn(async move {
crates/slskr/src/main.rs:18616:    let response = time::timeout(state.config.peer_response_timeout, peer.receive())
crates/slskr/src/main.rs:18678:    time::timeout(state.config.peer_response_timeout, peer.receive())
crates/slskr/src/main.rs:18688:    time::timeout(state.config.peer_response_timeout, peer.receive())
crates/slskr/src/main.rs:19426:    *next_wishlist_search = Instant::now() + scheduler.interval();
crates/slskr/src/main.rs:19970:    let stream = time::timeout(
crates/slskr/src/main.rs:19977:    time::timeout(
crates/slskr/src/main.rs:20187:    time::timeout(
crates/slskr/src/main.rs:20194:    let offset = time::timeout(
crates/slskr/src/main.rs:20221:        time::timeout(
crates/slskr/src/main.rs:20287:    let token = time::timeout(
crates/slskr/src/main.rs:20300:    time::timeout(
crates/slskr/src/main.rs:20311:        let chunk = time::timeout(
crates/slskr/src/main.rs:20350:    time::timeout(
crates/slskr/src/main.rs:20367:    let stream = time::timeout(
crates/slskr/src/main.rs:20377:    let stream = time::timeout(
crates/slskr/src/main.rs:20400:    let stream = time::timeout(
crates/slskr/src/main.rs:20407:    let stream = time::timeout(
crates/slskr/src/main.rs:20426:    let stream = time::timeout(
crates/slskr/src/main.rs:20433:    let stream = time::timeout(
crates/slskr/src/main.rs:20548:    time::timeout(
crates/slskr/src/main.rs:20555:    let message = time::timeout(state.config.peer_response_timeout, peer.receive())
crates/slskr/src/main.rs:20568:    time::timeout(
crates/slskr/src/main.rs:20578:    let message = time::timeout(state.config.peer_response_timeout, peer.receive())
crates/slskr/src/main.rs:20590:    let mut peer = time::timeout(timeout, connect_peer_messages(address, username))
crates/slskr/src/main.rs:20594:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/main.rs:20598:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20610:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr/src/main.rs:20614:    let stream = time::timeout(
crates/slskr/src/main.rs:20622:    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
crates/slskr/src/main.rs:20626:    let message = time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20687:    let mut peer = time::timeout(timeout, connect_peer_messages(address, username))
crates/slskr/src/main.rs:20691:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/main.rs:20695:    time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20707:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr/src/main.rs:20711:    let stream = time::timeout(
crates/slskr/src/main.rs:20719:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/main.rs:20723:    time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20736:    let mut peer = time::timeout(timeout, connect_peer_messages(address, username))
crates/slskr/src/main.rs:20740:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/main.rs:20744:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20758:    let stream = time::timeout(timeout, TcpStream::connect(address))
crates/slskr/src/main.rs:20762:    let stream = time::timeout(
crates/slskr/src/main.rs:20770:    time::timeout(timeout, peer.send(&message))
crates/slskr/src/main.rs:20774:    let response = time::timeout(timeout, peer.receive())
crates/slskr/src/main.rs:20806:            let queued = time::timeout(timeout, peer.receive_peer_message())
crates/slskr/src/main.rs:24069:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:24107:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:24283:        let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
crates/slskr/src/main.rs:26155:            let response = tokio::time::timeout(
crates/slskr/src/main.rs:26171:            let versioned_response = tokio::time::timeout(
crates/slskr/src/main.rs:26222:            let response = tokio::time::timeout(
crates/slskr/src/main.rs:26277:            let response = tokio::time::timeout(
crates/slskr/src/main.rs:26340:            let response = tokio::time::timeout(
crates/slskr/src/main.rs:29664:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:29741:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:29834:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:29932:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:30003:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:30099:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:30212:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:30268:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:30363:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:32508:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:32569:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:32658:        let server = tokio::spawn(async move {
crates/slskr/src/main.rs:33781:        let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);

## Browser injection, token storage, and opener boundaries
dashboard/src/hooks/useLocalStorage.ts:8:  storageName: 'localStorage' | 'sessionStorage',
dashboard/src/hooks/useLocalStorage.ts:42: * Custom hook for managing localStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:45:  return useBrowserStorage(key, initialValue, 'localStorage');
dashboard/src/hooks/useLocalStorage.ts:49: * Custom hook for managing sessionStorage with React state.
dashboard/src/hooks/useLocalStorage.ts:52:  return useBrowserStorage(key, initialValue, 'sessionStorage');
web/e2e/helpers.ts:254:        sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:255:        localStorage.getItem('slskr-token')
web/e2e/helpers.ts:273:        localStorage: Object.keys(localStorage).map((k) => ({
web/e2e/helpers.ts:275:          value: localStorage.getItem(k)?.slice(0, 50),
web/e2e/helpers.ts:277:        sessionStorage: Object.keys(sessionStorage).map((k) => ({
web/e2e/helpers.ts:279:          value: sessionStorage.getItem(k)?.slice(0, 50),
web/e2e/helpers.ts:482:        sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:483:        localStorage.getItem('slskr-token');
web/e2e/helpers.ts:662:      sessionStorage.getItem('slskr-token') ||
web/e2e/helpers.ts:663:      localStorage.getItem('slskr-token') ||
dashboard/src/pages/Monitoring.tsx:88:          target="_blank"
web/scripts/audit-react-webui.mjs:430:      window.localStorage.setItem('slskr-theme', 'slskr');
web/scripts/audit-react-webui.mjs:431:      window.sessionStorage.setItem('slskr-token', 'audit-token');
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
web/src/lib/safeOpen.js:2:  const opened = window.open(url, '_blank', 'noopener,noreferrer');
web/src/lib/session.js:13:  setToken(sessionStorage, tokenPassthroughValue);
web/src/lib/session.js:35:  setToken(sessionStorage, candidateToken);
web/src/lib/searches.js:26:// Blocked users management (localStorage-based)
web/src/components/Shared/Footer.jsx:218:                target="_blank"
web/src/components/Shared/Footer.jsx:234:                target="_blank"
web/src/components/Shared/Footer.jsx:245:                  target="_blank"
web/src/components/Shared/Footer.jsx:255:                target="_blank"
web/src/components/System/ExperienceSettings/index.jsx:86:    const stored = JSON.parse(localStorage.getItem(storageKey) || '{}');
web/src/components/System/ExperienceSettings/index.jsx:116:    localStorage.setItem(storageKey, JSON.stringify(form));
web/src/components/System/ExperienceSettings/index.jsx:121:    localStorage.removeItem(storageKey);
web/src/components/Chat/Chat.jsx:20:// Load tabs from localStorage
web/src/components/Chat/Chat.jsx:38:// Save tabs to localStorage
web/src/components/Chat/Chat.jsx:164:  // Save tabs to localStorage whenever they change
web/src/components/Browse/Browse.jsx:9:// Load tabs from localStorage
web/src/components/Browse/Browse.jsx:27:// Save tabs to localStorage
web/src/components/Browse/Browse.jsx:92:  // Save tabs to localStorage whenever they change
web/src/components/Rooms/Rooms.jsx:22:// Load tabs from localStorage
web/src/components/Rooms/Rooms.jsx:40:// Save tabs to localStorage
web/src/components/Rooms/Rooms.jsx:96:  // Save tabs to localStorage whenever they change
web/src/components/Search/Detail/SearchDetail.jsx:140:  // Sync hasSavedDefault across tabs/searches when localStorage changes

## Suppressed CI and script failures
scripts/run-certification.sh:126:        set +e
scripts/run-certification.sh:272:    set +e
scripts/run-certification.sh:289:    set +e
scripts/run-certification.sh:307:    set +e
scripts/run-certification.sh:350:        set +e
scripts/run-certification.sh:416:        server_ip="$(getent ahostsv4 server.slsknet.org 2>/dev/null | awk 'NR == 1 { print $1 }')" || true
scripts/run-certification.sh:425:    set +e
scripts/run-certification.sh:447:    set +e
scripts/run-certification.sh:469:    set +e
scripts/run-certification.sh:500:    set +e
scripts/run-certification.sh:514:    kill "$listener_pid" 2>/dev/null || true
scripts/run-certification.sh:515:    wait "$listener_pid" 2>/dev/null || true
scripts/run-certification.sh:550:    set +e
scripts/run-certification.sh:578:    set +e
scripts/run-certification.sh:605:    set +e
scripts/run-certification.sh:631:    set +e
scripts/run-certification.sh:658:    set +e
scripts/run-certification.sh:717:    set +e
scripts/run-certification.sh:746:    set +e
scripts/run-certification.sh:772:    set +e
scripts/run-certification.sh:798:    set +e
scripts/run-certification.sh:824:    set +e
scripts/run-certification.sh:849:    set +e
scripts/run-certification.sh:896:    set +e
scripts/run-certification.sh:924:    set +e
scripts/run-certification.sh:950:    set +e
scripts/run-certification.sh:976:    set +e
scripts/run-certification.sh:1017:    set +e
scripts/run-certification.sh:1048:    set +e
scripts/run-certification.sh:1081:    set +e
scripts/run-certification.sh:1112:    set +e
scripts/run-certification.sh:1141:        set +e
scripts/run-certification.sh:1198:    set +e
scripts/run-certification.sh:1228:    set +e
scripts/run-certification.sh:1260:        set +e
scripts/run-certification.sh:1291:            set +e
scripts/run-certification.sh:1327:    set +e
scripts/run-certification.sh:1360:        set +e
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
scripts/run-slskdn-cross-client-interop.sh:119:  set +e
scripts/run-slskdn-cross-client-interop.sh:271:slskdn_binary="$(discover_slskdn_binary || true)"
scripts/run-slskdn-cross-client-interop.sh:281:  slskdn_binary="$(discover_slskdn_binary || true)"
scripts/run-slskdn-cross-client-interop.sh:318:      kill "$pid" 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:319:      wait "$pid" 2>/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:404:      if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:412:  tail -n 120 "$slskr_log" >&2 || true
scripts/run-slskdn-cross-client-interop.sh:421:      if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
scripts/run-slskdn-cross-client-interop.sh:429:  tail -n 120 "$slskdn_log" >&2 || true
scripts/run-slskdn-cross-client-interop.sh:438:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:440:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:442:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" || true
scripts/run-slskdn-cross-client-interop.sh:444:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/run-slskdn-cross-client-interop.sh:447:try_request slskr-share-rescan auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" '{}' >/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:450:  || true
scripts/run-slskdn-cross-client-interop.sh:497:  if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:513:  if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
scripts/run-slskdn-cross-client-interop.sh:611:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:612:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:667:probe_peer_address slskr "$slskr_username" || true
scripts/run-slskdn-cross-client-interop.sh:668:probe_peer_address slskdn "$slskdn_username" || true
scripts/run-slskdn-cross-client-interop.sh:683:    status="$(printf '%s' "$transfer_json" | json_get status 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:684:    bytes="$(printf '%s' "$transfer_json" | json_get bytes_transferred 2>/dev/null || true)"
scripts/run-slskdn-cross-client-interop.sh:719:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:721:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:723:    auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/run-council-scan.sh:14:    "$@" >"$tmp" || true
scripts/build-rust-web.sh:16:wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
scripts/check-proton-wg-labels.sh:38:  set +e
scripts/generate-vpn-soulseek-accounts.sh:65:  grep -v -E '^(SLSKR_TEST_ACCOUNT_COUNT|SLSKR_TEST_[0-9]+_(USERNAME|PASSWORD))=' "$output_file" > "$tmp" || true
scripts/generate-vpn-soulseek-accounts.sh:78:  set +e
.github/workflows/release-publish.yml:231:            KRB5CCNAME="FILE:$armor" kdestroy || true
.github/workflows/release-publish.yml:338:            --jq '.commit.committer.date' 2>/dev/null | { read -r d && date -u -d "$d" +%s; } || true)"
.github/workflows/release-publish.yml:377:            getent ahosts ppa.launchpad.net || true
.github/workflows/release-publish.yml:420:            ssh-keyscan -T 30 -t rsa,ecdsa,ed25519 ppa.launchpad.net >> ~/.ssh/known_hosts 2>/dev/null || true
.github/workflows/release-publish.yml:528:        continue-on-error: true
scripts/check-public-posture.sh:21:      | rg -v -i 'do not|should not|must not|unless|avoid|remove casual|presenting the repository|not copied|not copy|not import|not say|prohibited|forbidden|current web ui as the reference implementation|based on error type' || true
scripts/run-live-interop-matrix.sh:42:  live_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }' || true)"
scripts/run-live-interop-matrix.sh:117:    tail -n 20 "$stderr_file" || true
scripts/run-live-interop-matrix.sh:134:  set +e
scripts/run-live-interop-matrix.sh:164:set +e
scripts/run-live-interop-matrix.sh:190:set +e
scripts/run-live-interop-matrix.sh:211:set +e
scripts/run-live-soak-proton-natpmp.sh:113:    trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
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
scripts/run-live-http-transfer-smoke.sh:127:      kill "$pid" 2>/dev/null || true
scripts/run-live-http-transfer-smoke.sh:128:      wait "$pid" 2>/dev/null || true
scripts/run-live-http-transfer-smoke.sh:181:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-live-http-transfer-smoke.sh:189:  tail -n 80 "$work_dir/$name.log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:204:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" && "${seen:-0}" -ge 6 ]]; then
scripts/run-live-http-transfer-smoke.sh:263:  tail -n 40 "$stdout_file" >&2 || true
scripts/run-live-http-transfer-smoke.sh:264:  tail -n 40 "$stderr_file" >&2 || true
scripts/run-live-http-transfer-smoke.sh:272:    auth_post_json "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse/request" '{}' >/dev/null || true
scripts/run-live-http-transfer-smoke.sh:276:        status="$(printf '%s' "$browse_json" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:277:        count="$(printf '%s' "$browse_json" | json_field count 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:292:  tail -n 80 "$target_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:316:  status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:317:  bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:323:    tail -n 80 "$source_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:324:    tail -n 80 "$target_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:330:status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:331:bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
scripts/run-live-http-transfer-smoke.sh:334:  tail -n 80 "$source_log" >&2 || true
scripts/run-live-http-transfer-smoke.sh:335:  tail -n 80 "$target_log" >&2 || true
scripts/run-proton-public-matrix.sh:120:    set +e
scripts/run-proton-public-matrix.sh:157:    set +e
scripts/run-proton-public-matrix.sh:182:                            natpmpc -g "${PROTON_NATPMP_GATEWAY:-10.2.0.1}" -a "$public_port" "$local_port" tcp 60 >/dev/null 2>&1 || true
scripts/run-proton-public-matrix.sh:188:                    trap "kill \"$renew_pid\" 2>/dev/null || true" EXIT
scripts/run-proton-public-matrix.sh:247:    wait_for_metadata "$listener" "$metadata_probe" || true
scripts/run-proton-natpmp-command.sh:35:    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
scripts/run-proton-natpmp-command.sh:42:trap 'kill "$renew_pid" 2>/dev/null || true' EXIT
scripts/start-proton-listener-soak.sh:20:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:21:sudo wg-quick down "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:22:sudo ip link del "$interface" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:23:sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/start-proton-listener-soak.sh:24:sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:34:    kill "$daemon_pid" 2>/dev/null || true
scripts/run-slskd-api-compat-smoke.sh:35:    wait "$daemon_pid" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:37:    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:38:    sudo ip netns del "$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:39:    sudo rm -rf "/etc/netns/$namespace" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:40:    sudo ip link del "$host_veth" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:42:        sudo ip route del "$endpoint_ip/32" 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:44:    sudo iptables -t nat -D POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:45:    sudo iptables -D FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:46:    sudo iptables -D FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
scripts/run-in-proton-wg-netns.sh:139:sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
