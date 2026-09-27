use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ffi::OsString,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use slskr_client::protocol::peer::{FileEntry, FileSearchResponse};
use tokio::sync::{mpsc, RwLock};

use crate::config::{json_escape, redact_username, ConfigEnv, FileConfig};
use crate::utils::{
    normalize_api_path, parse_route, percent_decode, query_params, split_request_target,
};

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn podcore_timespan_stats_preserve_target_millisecond_precision() {
    assert_eq!(super::format_timespan_millis(0), "00:00:00");
    assert_eq!(super::format_timespan_millis(1), "00:00:00.0010000");
    assert_eq!(super::format_timespan_millis(1_234), "00:00:01.2340000");
    assert_eq!(
        super::format_timespan_millis(86_400_001),
        "1.00:00:00.0010000"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn secure_oauth_state_fails_closed_when_randomness_is_unavailable() {
    assert!(super::secure_oauth_state_with(|_| false).is_none());

    let token = super::secure_oauth_state_with(|bytes| {
        bytes.fill(0xab);
        true
    })
    .expect("deterministic randomness fixture");
    assert_eq!(token, format!("slskr-{}", "ab".repeat(32)));

    assert!(super::secure_share_grant_token_with(|_| false).is_none());
    let share_token = super::secure_share_grant_token_with(|bytes| {
        bytes.fill(0xcd);
        true
    })
    .expect("deterministic randomness fixture");
    assert_eq!(share_token, format!("share-{}", "cd".repeat(32)));
}

#[derive(Clone, Default)]
struct MapEnv {
    values: BTreeMap<String, String>,
}

impl MapEnv {
    fn with(mut self, name: &str, value: &str) -> Self {
        self.values.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl ConfigEnv for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn default_controller_options_omit_unconfigured_optional_values() {
    for target in ["slskd", "slskdn"] {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-default-options-{target}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&state_dir).unwrap();
        let config = crate::config::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
                .with("SLSKR_CONTROLLER_PROFILE", target),
        )
        .unwrap();
        let overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
        let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
            &config, &overlay, true,
        ))
        .unwrap();

        assert!(options["retention"].get("search").is_none());
        assert_eq!(options["retention"]["files"], serde_json::json!({}));
        assert_eq!(
            options["retention"]["transfers"]["upload"],
            serde_json::json!({})
        );
        assert_eq!(
            options["retention"]["transfers"]["download"],
            serde_json::json!({})
        );
        assert!(options["shares"]["cache"].get("retention").is_none());
        if target == "slskd" {
            assert!(options["web"].get("socket").is_none());
        } else {
            assert_eq!(options["web"]["socket"], "");
            assert!(options["telemetry"]["tracing"]
                .get("jaegerEndpoint")
                .is_none());
            assert!(options["telemetry"]["tracing"].get("jaegerPort").is_none());
            assert!(options["telemetry"]["tracing"]
                .get("otlpEndpoint")
                .is_none());
        }
        fs::remove_dir_all(state_dir).unwrap();
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn soulseek_connector_uses_authenticated_socks5_proxy() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind SOCKS5 fixture");
    let port = listener.local_addr().expect("SOCKS5 address").port();
    let proxy = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept SOCKS5 client");
        let mut greeting = [0_u8; 4];
        stream.read_exact(&mut greeting).await.expect("greeting");
        assert_eq!(greeting, [0x05, 0x02, 0x00, 0x02]);
        stream.write_all(&[0x05, 0x02]).await.expect("method");

        let mut auth_header = [0_u8; 2];
        stream
            .read_exact(&mut auth_header)
            .await
            .expect("auth header");
        assert_eq!(auth_header, [0x01, 5]);
        let mut username = [0_u8; 5];
        stream.read_exact(&mut username).await.expect("username");
        assert_eq!(&username, b"alice");
        let mut password_len = [0_u8; 1];
        stream
            .read_exact(&mut password_len)
            .await
            .expect("password length");
        assert_eq!(password_len[0], 6);
        let mut password = [0_u8; 6];
        stream.read_exact(&mut password).await.expect("password");
        assert_eq!(&password, b"secret");
        stream
            .write_all(&[0x01, 0x00])
            .await
            .expect("auth response");

        let mut request_header = [0_u8; 5];
        stream
            .read_exact(&mut request_header)
            .await
            .expect("connect header");
        assert_eq!(request_header, [0x05, 0x01, 0x00, 0x03, 12]);
        let mut target = [0_u8; 12];
        stream.read_exact(&mut target).await.expect("target");
        assert_eq!(&target, b"example.test");
        let mut target_port = [0_u8; 2];
        stream
            .read_exact(&mut target_port)
            .await
            .expect("target port");
        assert_eq!(u16::from_be_bytes(target_port), 4242);
        stream
            .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
            .await
            .expect("connect response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SLSK_PROXY_ENABLED", "true")
            .with("SLSKR_SLSK_PROXY_ADDRESS", "127.0.0.1")
            .with("SLSKR_SLSK_PROXY_PORT", &port.to_string())
            .with("SLSKR_SLSK_PROXY_USERNAME", "alice")
            .with("SLSKR_SLSK_PROXY_PASSWORD", "secret"),
    );
    let stream = super::connect_soulseek_tcp(
        &state,
        "example.test:4242",
        super::SoulseekSocketClass::Control,
    )
    .await
    .expect("connect through SOCKS5");
    assert_eq!(stream.peer_addr().expect("proxy peer").port(), port);
    proxy.await.expect("SOCKS5 fixture task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn soulseek_connector_uses_no_auth_socks5_and_transfer_buffers() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind SOCKS5 fixture");
    let port = listener.local_addr().expect("SOCKS5 address").port();
    let proxy = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept SOCKS5 client");
        let mut greeting = [0_u8; 3];
        stream.read_exact(&mut greeting).await.expect("greeting");
        assert_eq!(greeting, [0x05, 0x01, 0x00]);
        stream.write_all(&[0x05, 0x00]).await.expect("method");
        let mut request = [0_u8; 10];
        stream
            .read_exact(&mut request)
            .await
            .expect("connect request");
        assert_eq!(request, [0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 8, 223]);
        stream
            .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
            .await
            .expect("connect response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SLSK_PROXY_ENABLED", "true")
            .with("SLSKR_SLSK_PROXY_ADDRESS", "127.0.0.1")
            .with("SLSKR_SLSK_PROXY_PORT", &port.to_string())
            .with("SLSKR_SLSK_TRANSFER_BUFFER", "81920"),
    );
    super::connect_soulseek_tcp(
        &state,
        "127.0.0.1:2271",
        super::SoulseekSocketClass::Transfer,
    )
    .await
    .expect("connect through no-auth SOCKS5");
    proxy.await.expect("SOCKS5 fixture task");
}

async fn serve_json_fixture(
    listener: &tokio::net::TcpListener,
    response: serde_json::Value,
) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = listener.accept().await.expect("accept fixture request");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let count = stream
            .read(&mut buffer)
            .await
            .expect("read fixture request");
        assert!(count > 0, "fixture request ended before headers");
        request.extend_from_slice(&buffer[..count]);
        if let Some(position) = request.windows(4).position(|row| row == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or_default();
    while request.len() < header_end.saturating_add(content_length) {
        let count = stream.read(&mut buffer).await.expect("read fixture body");
        assert!(count > 0, "fixture request ended before body");
        request.extend_from_slice(&buffer[..count]);
    }
    let body = response.to_string();
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(reply.as_bytes())
        .await
        .expect("write fixture response");
    String::from_utf8(request).expect("fixture request UTF-8")
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn managed_blacklist_runtime_caches_by_username_and_clears_on_replace() {
    let root = std::env::temp_dir().join(format!(
        "slskr-managed-blacklist-runtime-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("blacklist.txt");
    fs::write(&path, "127.0.0.1/32\n").unwrap();
    let enabled = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("enabled managed blacklist");
    let disabled = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_BLACKLIST", "false")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("disabled managed blacklist");

    let mut runtime = super::ManagedBlacklistRuntime::new(
        enabled.managed_blacklist,
        enabled.controller_profile,
        enabled.controller_case_sensitive_regex,
    );
    assert!(runtime.is_blacklisted(Some("Peer"), Some("127.0.0.1".parse().unwrap()), 100));
    assert!(!runtime.is_blacklisted(Some("peer"), None, 101));
    assert!(!runtime.is_blacklisted(Some("other"), Some("127.0.0.2".parse().unwrap()), 101));

    runtime.replace(
        disabled.managed_blacklist,
        disabled.controller_profile,
        disabled.controller_case_sensitive_regex,
    );
    assert!(!runtime.is_blacklisted(Some("peer"), Some("127.0.0.1".parse().unwrap()), 102));
    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn revoked_jwt_store_persists_and_reloads_across_restart() {
    let root = std::env::temp_dir().join(format!(
        "slskr-revoked-jwt-store-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let now = super::unix_timestamp();

    {
        let mut store = super::RevokedJwtStore::load(&root).expect("load JWT revocation store");
        assert!(!store.contains("token-1", now), "fresh store must be empty");
        store
            .revoke("token-1".to_owned(), now + 3_600, now)
            .expect("persist JWT revocation");
        assert!(store.contains("token-1", now));
    }

    // Simulate a restart: a fresh store loaded from the same state
    // directory must still honor the revocation.
    let mut reloaded = super::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
    assert!(
        reloaded.contains("token-1", now),
        "revocation must survive a restart"
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn revoked_jwt_store_prunes_expired_entries_on_reload() {
    let root = std::env::temp_dir().join(format!(
        "slskr-revoked-jwt-store-prune-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let now = super::unix_timestamp();

    {
        let mut store = super::RevokedJwtStore::load(&root).expect("load JWT revocation store");
        store
            .revoke("short-lived-token".to_owned(), now + 1, now)
            .expect("persist JWT revocation");
    }

    // Reload well after expiry: the entry must be pruned, not resurrected.
    let mut reloaded = super::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
    assert!(!reloaded.contains("short-lived-token", now + 100));

    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn blacklist_pattern_case_mode_applies_to_current_and_frozen_profiles() {
    let slskd = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
            .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$"),
    )
    .unwrap();
    let mut controller_runtime = super::ManagedBlacklistRuntime::new(
        slskd.managed_blacklist,
        slskd.controller_profile,
        slskd.controller_case_sensitive_regex,
    );
    assert!(!controller_runtime.is_blacklisted(Some("CaseUser"), None, 1));
    assert!(controller_runtime.is_blacklisted(Some("caseuser"), None, 1));

    let slskdn = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
            .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$"),
    )
    .unwrap();
    let mut native_runtime = super::ManagedBlacklistRuntime::new(
        slskdn.managed_blacklist,
        slskdn.controller_profile,
        slskdn.controller_case_sensitive_regex,
    );
    assert!(!native_runtime.is_blacklisted(Some("CaseUser"), None, 1));
    assert!(native_runtime.is_blacklisted(Some("caseuser"), None, 1));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_regex_supports_dotnet_backtracking_constructs() {
    let cases = [
        (r"foo(?=bar)", "foobar", true),
        (r"foo(?!bar)", "foobaz", true),
        (r"(?<=foo)bar", "foobar", true),
        (r"(?<!foo)bar", "bazbar", true),
        (r"^(a+)\1$", "aaaa", true),
        (r"^(?<word>\w+)\s+\k<word>$", "same same", true),
        (r"^(?>a|ab)c$", "abc", false),
        (r"^(a)?b(?(1)c|d)$", "abc", true),
        (r"^(a)?b(?(1)c|d)$", "bd", true),
    ];

    for (expression, value, expected) in cases {
        let matcher = super::ControllerRegex::compile_with_timeout(expression, true, None)
            .unwrap_or_else(|error| panic!("failed to compile {expression:?}: {error}"));
        assert_eq!(
            matcher.is_match(value),
            expected,
            "unexpected .NET-compatible match for {expression:?} against {value:?}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_regex_applies_global_case_mode_to_backreferences() {
    let insensitive =
        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", false, None)
            .expect("case-insensitive named backreference");
    let sensitive =
        super::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", true, None)
            .expect("case-sensitive named backreference");

    assert!(insensitive.is_match("abcABC"));
    assert!(!sensitive.is_match("abcABC"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_controller_regex_timeout_is_fail_closed() {
    let matcher = super::ControllerRegex::compile_with_timeout(
        r"^(?=(a+)+$).*$",
        true,
        Some(super::NATIVE_REGEX_MATCH_TIMEOUT),
    )
    .expect("pathological regex");
    let hostile_username = format!("{}!", "a".repeat(255));
    let started = Instant::now();

    assert!(!matcher.is_match(&hostile_username));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "regex timeout guard exceeded the bounded execution window"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn incoming_search_filters_honor_startup_case_mode() {
    let (insensitive, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret"),
    );
    assert!(super::build_file_search_response(&insensitive, 1, "SECRET")
        .await
        .is_none());

    let (sensitive, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    assert!(super::build_file_search_response(&sensitive, 1, "SECRET")
        .await
        .is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn incoming_public_search_sends_response_over_peer_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind incoming-search peer fixture");
    let peer_address = listener
        .local_addr()
        .expect("incoming-search peer fixture address");
    let peer_task = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept incoming-search peer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("incoming-search peer init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        let response = peer.receive().await.expect("incoming-search peer response");
        let super::PeerMessage::FileSearchResponse(response) = response else {
            panic!("incoming-search peer received the wrong message");
        };
        assert_eq!(response.username, "tester");
        assert_eq!(response.token, 19);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].filename, "Virtual/SECRET.flac");
    });

    let endpoint = format!("searcher={peer_address}");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", &endpoint),
    );

    let session_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind incoming-search session fixture");
    let session_address = session_listener
        .local_addr()
        .expect("incoming-search session fixture address");
    let session_client = tokio::net::TcpStream::connect(session_address);
    let session_server = session_listener.accept();
    let (session_client, session_server) = tokio::join!(session_client, session_server);
    let _session_client = session_client.expect("connect incoming-search session fixture");
    let (session_server, _) = session_server.expect("accept incoming-search session fixture");
    let mut session = slskr_client::server::ServerSession::new(
        slskr_client::stream::ServerConnection::new(session_server),
    );

    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &super::ServerMessage::FileSearchIncoming {
            username: "searcher".to_owned(),
            token: 19,
            query: "SECRET".to_owned(),
        },
    )
    .await;

    tokio::time::timeout(Duration::from_secs(2), peer_task)
        .await
        .expect("incoming-search peer response timed out")
        .expect("incoming-search peer task failed");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn raw_peer_search_request_responds_without_peer_username() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw-search peer fixture");
    let peer_address = listener
        .local_addr()
        .expect("raw-search peer fixture address");
    let peer_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept raw-search peer");
        tokio::io::AsyncWriteExt::write_u8(&mut stream, b'P')
            .await
            .expect("write raw peer kind");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
        peer.send(&super::PeerMessage::FileSearchRequest {
            token: 23,
            query: "SECRET".to_owned(),
        })
        .await
        .expect("send raw-search request");
        let response = tokio::time::timeout(Duration::from_secs(2), peer.receive())
            .await
            .expect("raw-search response timed out")
            .expect("raw-search response failed");
        let super::PeerMessage::FileSearchResponse(response) = response else {
            panic!("raw-search peer received the wrong message");
        };
        assert_eq!(response.token, 23);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].filename, "Virtual/SECRET.flac");
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42"),
    );
    let stream = tokio::net::TcpStream::connect(peer_address)
        .await
        .expect("connect raw-search peer fixture");
    let remote_address = stream
        .peer_addr()
        .expect("raw-search peer fixture remote address");
    let incoming = slskr_client::listener::demux_incoming(stream)
        .await
        .expect("demux raw-search peer");
    let super::IncomingConnection::PeerMessages(peer) = incoming else {
        panic!("expected raw peer-message connection");
    };
    super::handle_plain_peer_messages_with_address(&state, peer, None, Some(remote_address.ip()))
        .await
        .expect("handle raw-search peer");
    peer_task.await.expect("raw-search peer task failed");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_search_filters_preserve_reloaded_case_mode() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    assert!(state.config.controller_case_sensitive_regex);
    assert!(super::build_file_search_response(&state, 1, "SECRET")
        .await
        .is_some());

    let yaml = "flags:\n  case_sensitive_reg_ex: false\nfilters:\n  search:\n    request:\n      - secret|other\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
        ("SLSKR_CONTROLLER_PROFILE".to_owned(), "slskdn".to_owned()),
    ]);
    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert!(super::build_file_search_response(&state, 1, "SECRET")
        .await
        .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_filters_honor_startup_case_mode_against_original_paths() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-filter-case-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("SECRET.flac"), b"test").unwrap();
    let shared = root.display().to_string();

    let insensitive = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_SHARED_DIR", &shared)
            .with("SLSKD_SHARE_FILTER", "secret"),
    )
    .unwrap();
    assert!(super::build_share_index(&insensitive).entries.is_empty());

    let sensitive = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_SHARED_DIR", &shared)
            .with("SLSKD_SHARE_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    )
    .unwrap();
    assert_eq!(super::build_share_index(&sensitive).entries.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_library_fallback_uses_current_case_mode_for_share_filters(
) {
    let root = std::env::temp_dir().join(format!(
        "slskr-library-filter-case-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("SECRET.flac"), b"library").unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_SHARED_DIR", &root.display().to_string())
            .with("SLSKD_SHARE_FILTER", r"(?<=/)secret(?=\.flac$)")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    {
        let mut shares = state.shares.write().await;
        shares.entries.clear();
        shares.local_paths.clear();
    }

    let visible_response = super::route_http_request(
        "GET",
        "/api/v0/library/items?query=SECRET",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(visible_response.status, "200 OK");
    assert!(visible_response.content_type.contains("application/json"));
    let visible = serde_json::from_str::<serde_json::Value>(&visible_response.body).unwrap();
    assert_eq!(visible["items"].as_array().unwrap().len(), 1);
    assert_eq!(visible["items"][0]["fileName"], "SECRET.flac");
    assert!(visible["items"][0]["contentId"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    *state
        .controller_case_sensitive_regex
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = false;
    let filtered_response = super::route_http_request(
        "GET",
        "/api/v0/library/items?query=SECRET",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(filtered_response.status, "200 OK");
    assert!(filtered_response.content_type.contains("application/json"));
    let filtered = serde_json::from_str::<serde_json::Value>(&filtered_response.body).unwrap();
    assert!(filtered["items"].as_array().unwrap().is_empty());

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/library/items",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/library/items",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_items_case_filter_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn managed_blacklist_empty_peer_payloads_match_the_frozen_wire_shape() {
    let browse = super::build_empty_browse_payload().expect("empty browse payload");
    assert_eq!(
        super::decompress_zlib_payload(&browse).unwrap(),
        vec![0; 12]
    );

    let folder = super::build_folder_contents_payload(&[], 124, "share", Default::default())
        .expect("empty folder payload");
    let decoded = super::decompress_zlib_payload(&folder).unwrap();
    let mut reader = slskr_client::protocol::Reader::new(&decoded);
    assert_eq!(reader.read_u32_le().unwrap(), 124);
    assert_eq!(reader.read_string().unwrap(), "share");
    assert_eq!(reader.read_u32_le().unwrap(), 1);
    assert_eq!(reader.read_string().unwrap(), "share");
    assert_eq!(reader.read_u32_le().unwrap(), 0);
    assert!(reader.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_swagger_routes_follow_the_target_specific_startup_default() {
    let (slskd, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let disabled = super::route_http_request("GET", "/swagger/v0/swagger.json", None, "", &slskd)
        .await
        .expect("disabled slskd swagger route");
    assert_eq!(disabled.status, "404 Not Found");
    assert!(disabled.content_type.is_empty());
    assert!(disabled.body.is_empty());

    let (slskdn, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let enabled = super::route_http_request("GET", "/swagger/v0/swagger.json", None, "", &slskdn)
        .await
        .expect("enabled slskdN swagger route");
    assert_eq!(enabled.status, "200 OK");
    let spec: serde_json::Value = serde_json::from_str(&enabled.body).unwrap();
    assert_eq!(spec["openapi"], "3.0.4");
    assert_eq!(spec["info"]["title"], "slskr API");
    assert!(spec["paths"]
        .as_object()
        .is_some_and(|paths| !paths.is_empty()));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_metrics_route_uses_its_own_basic_authentication() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_URL", "prometheus")
            .with("SLSKD_METRICS_USERNAME", "metrics-user")
            .with("SLSKD_METRICS_PASSWORD", "metrics-pass"),
    );
    let missing = super::route_http_request("GET", "/prometheus", None, "", &state)
        .await
        .expect("missing metrics auth response");
    assert_eq!(missing.status, "401 Unauthorized");
    assert!(missing.body.is_empty());

    let malformed = super::route_http_request("GET", "/prometheus", Some("Basic !!!"), "", &state)
        .await
        .expect("malformed metrics auth response");
    assert_eq!(malformed.status, "401 Unauthorized");

    let authorized = super::route_http_request(
        "GET",
        "/prometheus",
        Some("Basic bWV0cmljcy11c2VyOm1ldHJpY3MtcGFzcw=="),
        "",
        &state,
    )
    .await
    .expect("authorized metrics response");
    assert_eq!(authorized.status, "200 OK");
    assert_eq!(
        authorized.content_type,
        "text/plain; version=0.0.4; charset=utf-8"
    );
    assert!(authorized.body.contains("# HELP slskr_session_connected"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_headless_suppresses_ui_but_retains_api_routes() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_HEADLESS", "true"),
    );
    for path in ["/", "/missing-client-route"] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("headless UI response");
        assert_eq!(response.status, "404 Not Found");
        assert!(response.body.is_empty());
    }
    let application = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("headless API response");
    assert_eq!(application.status, "200 OK");

    let login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"slskd","password":"slskd"}"#,
        &state,
    )
    .await
    .expect("headless login response");
    assert_eq!(login.status, "403 Forbidden");
    assert!(login.body.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn serve_arguments_reject_unknown_or_duplicate_options() {
    assert_eq!(
        super::parse_serve_args(&[OsString::from("serve")]),
        Ok(Some(super::ServeInvocation::default()))
    );
    assert_eq!(
        super::parse_serve_args(&[OsString::from("serve"), OsString::from("--once")]),
        Ok(Some(super::ServeInvocation {
            once: true,
            ..Default::default()
        }))
    );
    assert!(
        super::parse_serve_args(&[OsString::from("serve"), OsString::from("--ocne"),]).is_err()
    );
    assert!(super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--once"),
        OsString::from("--once"),
    ])
    .is_err());
    assert_eq!(
        super::parse_serve_args(&[OsString::from("probe")]),
        Ok(None)
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn regex_filter_arguments_preserve_frozen_multi_value_semantics() {
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--case-sensitive-regex"),
        OsString::from("--share-filter=first"),
        OsString::from("--share-filter"),
        OsString::from("second"),
        OsString::from("--search-request-filter=short"),
        OsString::from("--search-request-filter"),
        OsString::from("private"),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_CASE_SENSITIVE_REGEX"),
        Some(&"true".to_owned())
    );
    assert_eq!(
        invocation.config_environment.get("SLSKD_SHARE_FILTER"),
        Some(&"first;second".to_owned())
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SEARCH_REQUEST_FILTER"),
        Some(&"short;private".to_owned())
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn frozen_controller_command_line_options_map_to_exact_startup_names() {
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--debug"),
        OsString::from("--headless"),
        OsString::from("--remote-configuration"),
        OsString::from("--remote-file-management"),
        OsString::from("--no-connect"),
        OsString::from("-n"),
        OsString::from("-x"),
        OsString::from("--no-version-check"),
        OsString::from("--experimental"),
        OsString::from("--no-share-scan"),
        OsString::from("--force-share-scan"),
        OsString::from("--enable-blacklist"),
        OsString::from("--blacklist-file=/etc/slskd/blacklist.txt"),
        OsString::from("--swagger"),
        OsString::from("--metrics"),
        OsString::from("--metrics-url=prometheus"),
        OsString::from("--metrics-no-auth"),
        OsString::from("--metrics-username"),
        OsString::from("metrics-user"),
        OsString::from("--metrics-password=metrics-pass"),
        OsString::from("--http-ip-address=127.0.0.2"),
        OsString::from("--http-address=127.0.0.3"),
        OsString::from("--http-port"),
        OsString::from("55030"),
        OsString::from("--instance-name"),
        OsString::from("frozen-instance"),
        OsString::from("--downloads"),
        OsString::from("/srv/slskd/downloads"),
        OsString::from("--incomplete=/srv/slskd/incomplete"),
        OsString::from("--shared=/srv/slskd/music"),
        OsString::from("--slsk-address"),
        OsString::from("soulseek.example"),
        OsString::from("--slsk-port=2271"),
        OsString::from("--slsk-listen-port"),
        OsString::from("55031"),
        OsString::from("--slsk-obfuscation-mode=prefer"),
        OsString::from("--slsk-obfuscation-advertise-regular-port"),
        OsString::from("--download-completed-path-template"),
        OsString::from("{uploader}/{remote_folder}"),
    ])
    .expect("valid frozen controller flags")
    .expect("serve invocation");

    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_DEBUG")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HEADLESS")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_REMOTE_CONFIGURATION")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_REMOTE_FILE_MANAGEMENT")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_CONNECT")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_START")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_LOGO")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_VERSION_CHECK")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_EXPERIMENTAL")
            .map(String::as_str),
        Some("true")
    );
    for name in ["SLSKD_NO_SHARE_SCAN", "SLSKD_FORCE_SHARE_SCAN"] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some("true")
        );
    }
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_BLACKLIST")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_BLACKLIST_FILE")
            .map(String::as_str),
        Some("/etc/slskd/blacklist.txt")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SWAGGER")
            .map(String::as_str),
        Some("true")
    );
    for (name, expected) in [
        ("SLSKD_METRICS", "true"),
        ("SLSKD_METRICS_URL", "prometheus"),
        ("SLSKD_METRICS_NO_AUTH", "true"),
        ("SLSKD_METRICS_USERNAME", "metrics-user"),
        ("SLSKD_METRICS_PASSWORD", "metrics-pass"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected)
        );
    }
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_IP_ADDRESS")
            .map(String::as_str),
        Some("127.0.0.2")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_PORT")
            .map(String::as_str),
        Some("55030")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_ADDRESS")
            .map(String::as_str),
        Some("127.0.0.3")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_INSTANCE_NAME")
            .map(String::as_str),
        Some("frozen-instance")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_DOWNLOADS_DIR")
            .map(String::as_str),
        Some("/srv/slskd/downloads")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_INCOMPLETE_DIR")
            .map(String::as_str),
        Some("/srv/slskd/incomplete")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SHARED_DIR")
            .map(String::as_str),
        Some("/srv/slskd/music")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SLSK_OBFUSCATION_MODE")
            .map(String::as_str),
        Some("prefer")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT")
            .map(String::as_str),
        Some("true")
    );

    let direct = super::parse_serve_args(&[
        OsString::from("--slsk-address"),
        OsString::from("direct.example"),
    ])
    .expect("direct upstream invocation")
    .expect("direct invocation");
    assert_eq!(
        direct
            .config_environment
            .get("SLSKD_SLSK_ADDRESS")
            .map(String::as_str),
        Some("direct.example")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn frozen_ftp_command_line_options_map_to_exact_startup_names() {
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--ftp"),
        OsString::from("--ftp-address=ftp.example"),
        OsString::from("--ftp-port=2121"),
        OsString::from("--ftp-encryption-mode=Explicit"),
        OsString::from("--ftp-ignore-certificate-errors"),
        OsString::from("--ftp-username=fixture-user"),
        OsString::from("--ftp-password=fixture-password"),
        OsString::from("--ftp-remote-path=/incoming"),
        OsString::from("--ftp-overwrite-existing"),
        OsString::from("--ftp-connection-timeout=4321"),
        OsString::from("--ftp-retry-attempts=5"),
    ])
    .unwrap()
    .unwrap();
    for (name, expected) in [
        ("SLSKD_FTP", "true"),
        ("SLSKD_FTP_ADDRESS", "ftp.example"),
        ("SLSKD_FTP_PORT", "2121"),
        ("SLSKD_FTP_ENCRYPTION_MODE", "Explicit"),
        ("SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS", "true"),
        ("SLSKD_FTP_USERNAME", "fixture-user"),
        ("SLSKD_FTP_PASSWORD", "fixture-password"),
        ("SLSKD_FTP_REMOTE_PATH", "/incoming"),
        ("SLSKD_FTP_OVERWRITE_EXISTING", "true"),
        ("SLSKD_FTP_CONNECTION_TIMEOUT", "4321"),
        ("SLSKD_FTP_RETRY_ATTEMPTS", "5"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected),
            "{name}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn frozen_relay_command_line_options_map_to_exact_startup_names() {
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--relay"),
        OsString::from("--relay-mode=agent"),
        OsString::from("--controller-address=https://controller.example"),
        OsString::from("--controller-ignore-certificate-errors"),
        OsString::from("--controller-pinned-spki=pin-a,pin-b"),
        OsString::from("--controller-api-key=0123456789abcdef"),
        OsString::from("--controller-secret=abcdef0123456789"),
        OsString::from("--controller-downloads"),
    ])
    .unwrap()
    .unwrap();
    for (name, expected) in [
        ("SLSKD_RELAY", "true"),
        ("SLSKD_RELAY_MODE", "agent"),
        ("SLSKD_CONTROLLER_ADDRESS", "https://controller.example"),
        ("SLSKD_CONTROLLER_IGNORE_CERTIFICATE_ERRORS", "true"),
        ("SLSKD_CONTROLLER_PINNED_SPKI", "pin-a,pin-b"),
        ("SLSKD_CONTROLLER_API_KEY", "0123456789abcdef"),
        ("SLSKD_CONTROLLER_SECRET", "abcdef0123456789"),
        ("SLSKD_CONTROLLER_DOWNLOADS", "true"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected),
            "{name}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn private_message_auto_response_command_line_overrides_controller_yaml() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-auto-response-cli-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "soulseek:\n  private_message_auto_response:\n    enabled: false\n    message: yaml response\n    cooldown_minutes: 15\n",
    )
    .unwrap();
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--app-dir"),
        state_dir.clone().into_os_string(),
        OsString::from("--slsk-private-message-auto-response"),
        OsString::from("--slsk-private-message-auto-response-message"),
        OsString::from("cli response"),
        OsString::from("--slsk-private-message-auto-response-cooldown-minutes"),
        OsString::from("45"),
    ])
    .unwrap()
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &super::ControllerCliEnv {
            values: &invocation.config_environment,
        },
    )
    .unwrap();
    assert!(config.private_message_auto_response.enabled);
    assert_eq!(config.private_message_auto_response.message, "cli response");
    assert_eq!(config.private_message_auto_response.cooldown_minutes, 45);
    let mut overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    overlay.command_line_environment = invocation.config_environment;
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["soulseek"]["privateMessageAutoResponse"],
        serde_json::json!({
            "enabled": true,
            "message": "cli response",
            "cooldownMinutes": 45,
        })
    );
    let scalar_projection = super::controller_yaml_api_projection(serde_json::json!({
        "soulseek": {
            "private_message_auto_response": {
                "enabled": "true",
                "message": 123,
                "cooldown_minutes": "15",
            }
        }
    }));
    assert_eq!(
        scalar_projection["soulseek"]["privateMessageAutoResponse"],
        serde_json::json!({
            "enabled": true,
            "message": "123",
            "cooldownMinutes": 15,
        })
    );
    let auto_retry_projection = super::controller_yaml_api_projection(serde_json::json!({
        "transfers": {
            "download": {
                "auto_retry": {
                    "enabled": "false",
                    "retry_delay_seconds": "10",
                    "check_interval_seconds": null,
                    "max_attempts": "0",
                    "max_files_per_cycle": "2",
                    "max_files_per_peer_per_cycle": "2",
                    "peer_cooldown_seconds": "60",
                    "alternate_sources_enabled": "true",
                    "max_alternate_source_searches_per_cycle": null,
                    "alternate_source_size_tolerance_percent": "5.5",
                }
            }
        }
    }));
    assert_eq!(
        auto_retry_projection["global"]["download"]["autoRetry"],
        serde_json::json!({
            "enabled": false,
            "retryDelaySeconds": 10,
            "checkIntervalSeconds": 300,
            "maxAttempts": 0,
            "maxFilesPerCycle": 2,
            "maxFilesPerPeerPerCycle": 2,
            "peerCooldownSeconds": 60,
            "alternateSourcesEnabled": true,
            "maxAlternateSourceSearchesPerCycle": 1,
            "alternateSourceSizeTolerancePercent": 5.5,
        })
    );
    let _ = fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn startup_flag_command_line_projection_overrides_controller_yaml() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-startup-flags-cli-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "flags:\n  no_logo: false\n  no_start: false\n  no_version_check: false\n  experimental: false\nweb:\n  authentication:\n    passthrough:\n      allowed_cidrs: 127.0.0.1/32\n",
    )
    .unwrap();
    let invocation = super::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--app-dir"),
        state_dir.clone().into_os_string(),
        OsString::from("--no-logo"),
        OsString::from("--no-start"),
        OsString::from("--no-version-check"),
        OsString::from("--experimental"),
        OsString::from("--no-auth"),
        OsString::from("--enforce-security"),
        OsString::from("--allow-remote-no-auth"),
    ])
    .unwrap()
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &super::ControllerCliEnv {
            values: &invocation.config_environment,
        },
    )
    .unwrap();
    assert!(config.controller_no_logo);
    assert!(config.controller_no_start);
    assert!(config.controller_no_version_check);
    assert!(config.controller_experimental);
    assert!(!config.auth_required);
    assert!(config.controller_web_enforce_security);
    assert!(config.controller_web_allow_remote_no_auth);
    let mut overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    overlay.command_line_environment = invocation.config_environment;
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(options["flags"]["noLogo"], true);
    assert_eq!(options["flags"]["noStart"], true);
    assert_eq!(options["flags"]["noVersionCheck"], true);
    assert_eq!(options["flags"]["experimental"], true);
    assert_eq!(options["web"]["authentication"]["disabled"], true);
    assert_eq!(options["web"]["enforceSecurity"], true);
    assert_eq!(options["web"]["allowRemoteNoAuth"], true);
    fs::remove_dir_all(state_dir).unwrap();
}

fn test_state() -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    test_state_with_env(MapEnv::default())
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn reconnect_backoff_is_interrupted_by_session_commands() {
    let (sender, mut receiver) = mpsc::channel(1);
    let waiter = tokio::spawn(async move {
        super::wait_for_reconnect_or_command(&mut receiver, Duration::from_secs(3_600)).await
    });
    tokio::task::yield_now().await;
    sender
        .send(super::SessionCommand::Disconnect)
        .await
        .expect("send disconnect");

    let wake = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("command must interrupt reconnect delay")
        .expect("wait task");
    assert!(matches!(
        wake,
        super::ReconnectWake::Command(super::SessionCommand::Disconnect)
    ));
}

fn test_state_with_env(
    extra_env: MapEnv,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    test_state_with_env_parts(extra_env, super::SearchStore::new(), None)
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn role_denials_are_distinguishable_from_csrf_rejections() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token"),
    );
    state.session.write().await.state = "connected";
    // Authenticated, but with a role too low for this route -- must not
    // be reported as a CSRF rejection.
    let role_denied = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        Some("Bearer read-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(role_denied.status, "403 Forbidden");
    assert_eq!(
        role_denied.body,
        "{\"error\":\"insufficient permissions for this route\"}"
    );

    // Bearer credentials bypass cookie CSRF validation, matching the
    // frozen ValidateCsrfForCookiesOnly policy.
    let csrf_denied = super::route_http_request_with_headers(
        "POST",
        "/api/v0/searches",
        Some("Bearer write-token"),
        r#"{"searchText":"csrf-audit"}"#,
        &state,
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("https://evil.example".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(csrf_denied.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn session_create_does_not_echo_api_token() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "secret-token"),
    );
    let response = super::route_http_request(
        "POST",
        "/api/session",
        Some("Bearer secret-token"),
        r#"{"username":"user"}"#,
        &state,
    )
    .await
    .expect("session response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["token"], "");
    assert!(json["issued"].is_number());
    assert!(json["notBefore"].is_number());
    assert!(json["expires"].is_number());
    assert_eq!(json["tokenConfigured"], true);
    assert!(!response.body.contains("secret-token"));
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_session_issue_and_revoke() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "secret-token"),
    );
    let login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"secret-token"}"#,
        &state,
    )
    .await
    .expect("admin login");
    assert_eq!(login.status, "200 OK", "{}", login.body);
    let login = serde_json::from_str::<serde_json::Value>(&login.body).unwrap();
    assert_eq!(login["name"], "admin");
    assert_eq!(login["tokenType"], "Bearer");
    assert!(login["expires"].as_u64().unwrap() > login["issued"].as_u64().unwrap());
    let token = login["token"].as_str().unwrap();
    assert_eq!(token.split('.').count(), 3);
    let authorization = format!("Bearer {token}");

    let authorized = super::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("JWT-authorized request");
    assert_eq!(authorized.status, "200 OK", "{}", authorized.body);

    let mut tampered_token = token.to_owned();
    let replacement = if tampered_token.ends_with('a') {
        'b'
    } else {
        'a'
    };
    tampered_token.pop();
    tampered_token.push(replacement);
    let tampered_authorization = format!("Bearer {tampered_token}");
    let tampered = super::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&tampered_authorization),
        "",
        &state,
    )
    .await
    .expect("tampered JWT request");
    assert_eq!(tampered.status, "401 Unauthorized", "{}", tampered.body);

    let logout = super::route_http_request(
        "DELETE",
        "/api/v0/session",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("JWT logout");
    assert_eq!(logout.status, "204 No Content", "{}", logout.body);

    let revoked = super::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("revoked JWT request");
    assert_eq!(revoked.status, "401 Unauthorized", "{}", revoked.body);

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/session",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/session",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/session",
            "case": "mutation-side-effects-and-readback",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("session_issue_and_revoke.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_server_session_open_cases() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);

    let (session_state, _session_receiver) = test_state_with_env(env.clone());
    let session_malformed =
        super::route_http_request("GET", "/api/v0/session/malformed", None, "", &session_state)
            .await
            .expect("malformed session path");
    record!(
        "GET",
        "/api/v0/session",
        "malformed-path-query-or-body",
        session_malformed.status == "404 Not Found"
    );
    let session_empty =
        super::route_http_request("GET", "/api/v0/session", None, "", &session_state)
            .await
            .expect("empty session");
    let session_empty_json =
        serde_json::from_str::<serde_json::Value>(&session_empty.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session_empty.status == "200 OK" && session_empty_json["state"] == "disconnected"
    );
    session_state.session.write().await.state = "connected";
    session_state.session.write().await.username = Some("session-open-user".to_owned());
    let session_populated =
        super::route_http_request("GET", "/api/v0/session", None, "", &session_state)
            .await
            .expect("populated session");
    let session_populated_json =
        serde_json::from_str::<serde_json::Value>(&session_populated.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "populated-dynamic-state",
        session_populated.status == "200 OK"
            && session_populated_json["state"] == "connected"
            && session_populated_json["username"] == "session-open-user"
    );

    let session_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("session runtime database");
    let (session_runtime_state, _session_runtime_receiver) = test_state_with_env_parts(
        env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(session_runtime_db.clone()),
    );
    session_runtime_db.close_for_test().await;
    let session_runtime =
        super::route_http_request("GET", "/api/v0/session", None, "", &session_runtime_state)
            .await
            .expect("runtime session");
    record!(
        "GET",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        session_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session_runtime.body)
                .is_ok_and(|value| value.is_object())
    );

    let enabled_malformed = super::route_http_request(
        "GET",
        "/api/v0/session/enabled/extra",
        None,
        "",
        &session_state,
    )
    .await
    .expect("malformed session-enabled path");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "malformed-path-query-or-body",
        enabled_malformed.status == "404 Not Found"
    );
    let enabled_empty =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &session_state)
            .await
            .expect("empty session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "missing-empty-or-conflict-state",
        enabled_empty.status == "200 OK" && enabled_empty.body == "false"
    );
    let enabled_runtime = super::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        None,
        "",
        &session_runtime_state,
    )
    .await
    .expect("runtime session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "runtime-failure-and-timeout",
        enabled_runtime.status == "200 OK" && enabled_runtime.body == "false"
    );
    let enabled_env = env
        .clone()
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "session-open-api-token");
    let (enabled_state, _enabled_receiver) = test_state_with_env(enabled_env);
    let enabled_populated =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &enabled_state)
            .await
            .expect("populated session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "populated-dynamic-state",
        enabled_populated.status == "200 OK" && enabled_populated.body == "true"
    );

    let login_env = env
        .clone()
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "session-open-api-token")
        .with("SLSKD_USERNAME", "session-open-admin")
        .with("SLSKD_PASSWORD", "session-open-password");
    let (login_state, _login_receiver) = test_state_with_env(login_env.clone());
    let malformed_login_state = test_state_with_env(env.clone()).0;
    let malformed_login = super::route_http_request(
        "POST",
        "/api/v0/session/extra",
        None,
        r#"{"username":"session-open-admin","password":"session-open-password"}"#,
        &malformed_login_state,
    )
    .await
    .expect("malformed login path");
    record!(
        "POST",
        "/api/v0/session",
        "malformed-path-query-or-body",
        malformed_login.status == "404 Not Found"
    );
    let login_body = r#"{"username":"session-open-admin","password":"session-open-password"}"#;
    let login =
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state)
            .await
            .expect("session login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    let login_token = login_json["token"].as_str().unwrap_or_default().to_owned();
    let authorization = format!("Bearer {login_token}");
    let authorized_session = super::route_http_request(
        "GET",
        "/api/v0/session",
        Some(&authorization),
        "",
        &login_state,
    )
    .await
    .expect("authorized session readback");
    record!(
        "POST",
        "/api/v0/session",
        "mutation-side-effects-and-readback",
        login.status == "200 OK"
            && login_json["tokenType"] == "Bearer"
            && login_token.split('.').count() == 3
            && authorized_session.status == "200 OK"
    );
    let restarted_login_state = test_state_with_env(login_env.clone()).0;
    let restarted_login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        login_body,
        &restarted_login_state,
    )
    .await
    .expect("restarted session login");
    record!(
        "POST",
        "/api/v0/session",
        "restart-persistence-or-reset",
        restarted_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&restarted_login.body)
                .is_ok_and(|value| value["tokenType"] == "Bearer")
    );
    let login_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("login runtime database");
    let (login_runtime_state, _login_runtime_receiver) = test_state_with_env_parts(
        login_env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(login_runtime_db.clone()),
    );
    login_runtime_db.close_for_test().await;
    let runtime_login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        login_body,
        &login_runtime_state,
    )
    .await
    .expect("runtime session login");
    record!(
        "POST",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        runtime_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_login.body)
                .is_ok_and(|value| value["tokenType"] == "Bearer")
    );
    let concurrent_logins = futures_util::future::join_all([
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
    ])
    .await;
    let concurrent_tokens = concurrent_logins
        .iter()
        .filter_map(|response| response.as_ref().ok())
        .filter_map(|response| serde_json::from_str::<serde_json::Value>(&response.body).ok())
        .filter_map(|value| value["token"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    record!(
        "POST",
        "/api/v0/session",
        "concurrency-and-idempotency",
        concurrent_logins.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_tokens.len() == 2
    );

    let session_delete_malformed =
        super::route_http_request("DELETE", "/api/v0/session/extra", None, "", &session_state)
            .await
            .expect("malformed session delete path");
    record!(
        "DELETE",
        "/api/v0/session",
        "malformed-path-query-or-body",
        session_delete_malformed.status == "404 Not Found"
    );
    let session_delete_empty =
        super::route_http_request("DELETE", "/api/v0/session", None, "", &session_state)
            .await
            .expect("empty session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session_delete_empty.status == "204 No Content" && session_delete_empty.body.is_empty()
    );
    let session_delete_runtime = super::route_http_request(
        "DELETE",
        "/api/v0/session",
        None,
        "",
        &session_runtime_state,
    )
    .await
    .expect("runtime session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        session_delete_runtime.status == "204 No Content"
    );
    let restarted_session_delete_state = test_state_with_env(env.clone()).0;
    let restarted_session_delete = super::route_http_request(
        "DELETE",
        "/api/v0/session",
        None,
        "",
        &restarted_session_delete_state,
    )
    .await
    .expect("restarted session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "restart-persistence-or-reset",
        restarted_session_delete.status == "204 No Content"
    );
    let concurrent_session_deletes = futures_util::future::join_all([
        super::route_http_request("DELETE", "/api/v0/session", None, "", &session_state),
        super::route_http_request("DELETE", "/api/v0/session", None, "", &session_state),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/session",
        "concurrency-and-idempotency",
        concurrent_session_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let (server_state, mut server_receiver) = test_state_with_env(env.clone());
    let server_malformed =
        super::route_http_request("GET", "/api/v0/server/extra", None, "", &server_state)
            .await
            .expect("malformed server path");
    record!(
        "GET",
        "/api/v0/server",
        "malformed-path-query-or-body",
        server_malformed.status == "404 Not Found"
    );
    let server_status_malformed =
        super::route_http_request("GET", "/api/server/status/extra", None, "", &server_state)
            .await
            .expect("malformed server status path");
    record!(
        "GET",
        "/api/server/status",
        "malformed-path-query-or-body",
        server_status_malformed.status == "404 Not Found"
    );
    let server_status =
        super::route_http_request("GET", "/api/server/status", None, "", &server_state)
            .await
            .expect("empty server status");
    let server_status_json =
        serde_json::from_str::<serde_json::Value>(&server_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/server/status",
        "missing-empty-or-conflict-state",
        server_status.status == "200 OK"
            && server_status_json["connected"] == false
            && server_status_json["state"] == "disconnected"
            && server_status_json["username"] == ""
    );
    let (server_put_missing_state, _server_put_missing_receiver) = test_state_with_env(env.clone());
    let server_put_missing =
        super::route_http_request("PUT", "/api/v0/server", None, "", &server_put_missing_state)
            .await
            .expect("empty server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "missing-empty-or-conflict-state",
        server_put_missing.status == "200 OK"
            && server_put_missing_state.session.read().await.state == "connecting"
    );
    let server_put_malformed =
        super::route_http_request("PUT", "/api/v0/server/extra", None, "", &server_state)
            .await
            .expect("malformed server connect path");
    record!(
        "PUT",
        "/api/v0/server",
        "malformed-path-query-or-body",
        server_put_malformed.status == "404 Not Found"
    );
    let server_connect =
        super::route_http_request("PUT", "/api/v0/server", None, "", &server_state)
            .await
            .expect("server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        server_connect.status == "200 OK"
            && server_connect.body.is_empty()
            && server_state.session.read().await.state == "connecting"
            && matches!(
                server_receiver.try_recv(),
                Ok(super::SessionCommand::Connect)
            )
    );
    let (concurrent_server_put_state, _concurrent_server_put_receiver) =
        test_state_with_env(env.clone());
    let first_server_put = super::route_http_request(
        "PUT",
        "/api/v0/server",
        None,
        "",
        &concurrent_server_put_state,
    )
    .await
    .expect("first concurrent server connect");
    let concurrent_server_puts = futures_util::future::join_all([
        super::route_http_request(
            "PUT",
            "/api/v0/server",
            None,
            "",
            &concurrent_server_put_state,
        ),
        super::route_http_request(
            "PUT",
            "/api/v0/server",
            None,
            "",
            &concurrent_server_put_state,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/server",
        "concurrency-and-idempotency",
        first_server_put.status == "200 OK"
            && concurrent_server_puts.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "205 Reset Content")
            })
    );
    let (restarted_server_state, _restarted_server_receiver) = test_state_with_env(env.clone());
    let restarted_server_put =
        super::route_http_request("PUT", "/api/v0/server", None, "", &restarted_server_state)
            .await
            .expect("restarted server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "restart-persistence-or-reset",
        restarted_server_put.status == "200 OK"
            && restarted_server_state.session.read().await.state == "connecting"
    );
    let (put_failure_state, put_failure_receiver) = test_state_with_env(env.clone());
    drop(put_failure_receiver);
    let put_failure =
        super::route_http_request("PUT", "/api/v0/server", None, "", &put_failure_state)
            .await
            .expect("server connect runtime failure");
    record!(
        "PUT",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        put_failure.status == "503 Service Unavailable"
            && put_failure_state.session.read().await.state == "disconnected"
    );

    let (delete_server_state, mut delete_server_receiver) = test_state_with_env(env.clone());
    delete_server_state.session.write().await.state = "connected";
    let delete_server =
        super::route_http_request("DELETE", "/api/v0/server", None, "", &delete_server_state)
            .await
            .expect("server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "nominal-status-headers-body",
        delete_server.status == "204 No Content" && delete_server.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        delete_server_state.session.read().await.state == "disconnecting"
            && matches!(
                delete_server_receiver.try_recv(),
                Ok(super::SessionCommand::Disconnect)
            )
    );
    let delete_malformed =
        super::route_http_request("DELETE", "/api/v0/server", None, "{", &server_state)
            .await
            .expect("malformed server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_empty_state = test_state_with_env(env.clone()).0;
    let delete_empty =
        super::route_http_request("DELETE", "/api/v0/server", None, "", &delete_empty_state)
            .await
            .expect("empty server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "missing-empty-or-conflict-state",
        delete_empty.status == "204 No Content"
    );
    let (delete_failure_state, delete_failure_receiver) = test_state_with_env(env.clone());
    delete_failure_state.session.write().await.state = "connected";
    drop(delete_failure_receiver);
    let delete_failure =
        super::route_http_request("DELETE", "/api/v0/server", None, "", &delete_failure_state)
            .await
            .expect("server disconnect runtime failure");
    record!(
        "DELETE",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        delete_failure.status == "503 Service Unavailable"
            && delete_failure_state.session.read().await.state == "connected"
    );
    let restarted_server_delete_state = test_state_with_env(env.clone()).0;
    let restarted_server_delete = super::route_http_request(
        "DELETE",
        "/api/v0/server",
        None,
        "",
        &restarted_server_delete_state,
    )
    .await
    .expect("restarted server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "restart-persistence-or-reset",
        restarted_server_delete.status == "204 No Content"
    );
    let (concurrent_delete_state, _concurrent_delete_receiver) = test_state_with_env(env.clone());
    concurrent_delete_state.session.write().await.state = "connected";
    let concurrent_server_deletes = futures_util::future::join_all([
        super::route_http_request(
            "DELETE",
            "/api/v0/server",
            None,
            "",
            &concurrent_delete_state,
        ),
        super::route_http_request(
            "DELETE",
            "/api/v0/server",
            None,
            "",
            &concurrent_delete_state,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/server",
        "concurrency-and-idempotency",
        concurrent_server_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let server_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("server runtime database");
    let (server_runtime_state, _server_runtime_receiver) = test_state_with_env_parts(
        env,
        super::SearchStore::new(),
        Some(server_runtime_db.clone()),
    );
    server_runtime_db.close_for_test().await;
    let server_runtime =
        super::route_http_request("GET", "/api/v0/server", None, "", &server_runtime_state)
            .await
            .expect("runtime server state");
    let server_runtime_json =
        serde_json::from_str::<serde_json::Value>(&server_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        server_runtime.status == "200 OK"
            && server_runtime_json["address"] == ""
            && server_runtime_json["ipEndPoint"] == "255.255.255.255:0"
    );
    let server_status_runtime =
        super::route_http_request("GET", "/api/server/status", None, "", &server_runtime_state)
            .await
            .expect("runtime server status");
    let server_status_runtime_json =
        serde_json::from_str::<serde_json::Value>(&server_status_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/server/status",
        "runtime-failure-and-timeout",
        server_status_runtime.status == "200 OK"
            && server_status_runtime_json["connected"] == false
            && server_status_runtime_json["state"] == "disconnected"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create server/session evidence directory");
    fs::write(
        evidence_dir.join("server_session_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize server/session ledger"),
    )
    .expect("write server/session ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api server/session mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_session_bounds_failed_login_attempts() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token"),
    );
    for _ in 0..5 {
        let rejected = super::route_http_request(
            "POST",
            "/api/v0/session",
            None,
            r#"{"username":"admin","password":"wrong"}"#,
            &state,
        )
        .await
        .expect("failed login");
        assert_eq!(rejected.status, "401 Unauthorized");
    }
    let locked = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"secret-token"}"#,
        &state,
    )
    .await
    .expect("locked login");
    assert_eq!(locked.status, "429 Too Many Requests", "{}", locked.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_get_validation_and_missing_resource_statuses_match_slskdn() {
    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/collections/not-a-uuid",
        "/api/v0/share-grants/not-a-uuid",
        "/api/v0/multisource/search",
        "/api/v0/podcore/content/search",
        "/api/v0/telemetry/reports/transfers/leaderboard",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned GET validation");
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
        );
    }
    for path in [
        "/api/v0/conversations/missing",
        "/api/v0/conversations/missing/messages",
        "/api/v0/jobs/missing",
        "/api/v0/profile/missing",
        "/api/v0/searches/missing/responses",
        "/api/v0/security/adversarial",
        "/api/v0/security/canaries",
        "/api/v0/security/tor/status",
        "/api/v0/shares/missing",
        "/api/v0/transfers/downloads/missing",
        "/api/v0/users/missing/browse/status",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned missing GET");
        let expected_status = if path == "/api/v0/searches/missing/responses" {
            "400 Bad Request"
        } else {
            "404 Not Found"
        };
        assert_eq!(
            response.status, expected_status,
            "{path}: {}",
            response.body
        );
    }
    for path in [
        "/api/v0/relay/controller/downloads/token",
        "/api/v0/soulseek/mesh-rendezvous/discover",
        "/api/v0/soulseek/mesh-rendezvous/users",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("disabled versioned GET");
        assert_eq!(
            response.status, "403 Forbidden",
            "{path}: {}",
            response.body
        );
    }
    let response = super::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("native mesh health GET");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    for path in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("supported native versioned GET");
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn health_warns_when_auth_disabled_on_non_loopback() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
            .with("SLSKR_AUTH_DISABLED", "true"),
    );

    let response = super::route_http_request("GET", "/api/health", None, "", &state)
        .await
        .expect("health response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["warnings"][0], "auth_disabled_non_loopback");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_and_signal_routes_honor_configured_runtime_switches() {
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": false,
            "enableDht": false,
            "enableStun": false
        },
        "SignalSystem": {
            "enabled": false,
            "deduplicationCacheSize": 2048,
            "defaultTtl": "00:07:30",
            "meshChannel": {
                "enabled": false,
                "priority": 3,
                "requireActiveSession": true
            },
            "btExtensionChannel": {
                "enabled": true,
                "priority": 4,
                "requireActiveSession": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let config = super::route_http_request("GET", "/api/signals/config", None, "", &state)
        .await
        .expect("signal config response");
    let config_json = serde_json::from_str::<serde_json::Value>(&config.body).unwrap();
    assert!(!config_json["enabled"].as_bool().unwrap());
    assert_eq!(config_json["deduplicationCacheSize"], 2_048);
    assert_eq!(config_json["defaultTtlSeconds"], 450);
    assert!(!config_json["meshChannel"]["enabled"].as_bool().unwrap());
    assert_eq!(config_json["btExtensionChannel"]["priority"], 4);

    let status = super::route_http_request("GET", "/api/signals/status", None, "", &state)
        .await
        .expect("signal status response");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(
        status_json["activeChannels"],
        serde_json::json!(["btExtension"])
    );

    for (method, path) in [
        ("GET", "/api/mesh/stats"),
        ("GET", "/api/dht/peers"),
        ("POST", "/api/mesh/nat/detect"),
    ] {
        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .expect("disabled mesh route response");
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn root_health_endpoints_are_served_anonymously() {
    // Matches the oracle's endpoints.MapHealthChecks("/health") and
    // ("/health/mesh"), both AllowAnonymous -- orchestrator/container
    // health probes hit these at the root, not under /api.
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_AUTH_DISABLED", "false"));

    let health = super::route_http_request("GET", "/health", None, "", &state)
        .await
        .expect("root health response");
    assert_eq!(health.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap();
    assert_eq!(json["status"], "ok");

    let mesh_health = super::route_http_request("GET", "/health/mesh", None, "", &state)
        .await
        .expect("root mesh health response");
    assert_eq!(mesh_health.status, "200 OK");
    let mesh_json = serde_json::from_str::<serde_json::Value>(&mesh_health.body).unwrap();
    assert!(mesh_json.get("status").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn initiate_graceful_shutdown_disconnects_the_session() {
    // Matches the oracle's StopAsync teardown (Client.Disconnect before
    // exit) -- this is the logic a SIGTERM/SIGINT/SIGQUIT handler runs;
    // registering real OS signals isn't exercised here since raising
    // them would affect the whole shared test process.
    let (state, mut receiver) = test_state();
    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    super::initiate_graceful_shutdown(&state).await;
    let command = receiver.recv().await.expect("session command sent");
    assert!(matches!(command, super::SessionCommand::Disconnect));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn no_auth_mode_uses_native_passthrough_peer_identity() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "")
            .with("SLSK_PASSWORD", "")
            .with("SLSKR_AUTH_DISABLED", "true"),
    );

    assert_eq!(
        super::pod_request_peer_id(&state).await.as_deref(),
        Some("Anonymous")
    );

    let created = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-passthrough","name":"Passthrough","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod with passthrough identity");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let members = super::route_http_request(
        "GET",
        "/api/v0/pods/pod-passthrough/members",
        None,
        "",
        &state,
    )
    .await
    .expect("list passthrough pod members");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&members.body).unwrap()[0]["peerId"],
        "Anonymous"
    );

    let content_pod = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"Content Pod","contentId":"content:music:recording:passthrough"}"#,
        &state,
    )
    .await
    .expect("create content pod with passthrough identity");
    assert_eq!(content_pod.status, "201 Created", "{}", content_pod.body);
    assert!(state
        .pods
        .read()
        .await
        .can_moderate("pod:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "Anonymous"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_no_auth_passthrough_is_loopback_or_explicit_cidr_only() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
            .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "192.0.2.0/24"),
    )
    .expect("no-auth passthrough config");
    let check = |address: &str, path: &str| {
        super::routing::check_route_auth(
            &config,
            "GET",
            path,
            None,
            &super::RequestSecurityHeaders {
                remote_addr: Some(address.parse().unwrap()),
                ..Default::default()
            },
        )
    };

    assert_eq!(check("127.0.0.1:1", "/api/v0/application"), Ok(()));
    assert_eq!(check("192.0.2.25:1", "/api/v0/application"), Ok(()));
    assert_eq!(
        check("198.51.100.25:1", "/api/v0/application"),
        Err("unauthorized")
    );
    assert_eq!(check("198.51.100.25:1", "/api/health"), Ok(()));
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn state_directory_is_private_and_rejects_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-private-state-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let state_dir = root.join("state");
    super::ensure_private_state_dir(&state_dir).expect("create private state dir");
    assert_eq!(
        std::fs::metadata(&state_dir)
            .expect("state metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );

    let linked_state = root.join("linked-state");
    symlink(&state_dir, &linked_state).expect("create state symlink");
    let error =
        super::ensure_private_state_dir(&linked_state).expect_err("state symlink must be rejected");
    assert!(error.contains("must be a real directory"), "{error}");

    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn integration_json_reader_rejects_declared_oversized_response() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept fixture request");
        let mut request = [0_u8; 1024];
        let request_bytes = stream
            .read(&mut request)
            .await
            .expect("read fixture request");
        assert!(request_bytes > 0, "fixture request was empty");
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    super::MAX_INTEGRATION_RESPONSE_BYTES + 1
                )
                .as_bytes(),
            )
            .await
            .expect("write fixture response");
    });

    let response = reqwest::get(format!("http://{address}/oversized"))
        .await
        .expect("receive fixture headers");
    let error = super::read_bounded_integration_json(response, "fixture")
        .await
        .expect_err("oversized response must be rejected");
    assert!(error.contains("response exceeds"), "{error}");
    server.await.expect("fixture server task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn integration_json_reader_rejects_chunked_oversized_response() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept fixture request");
        let mut request = [0_u8; 1024];
        let request_bytes = stream
            .read(&mut request)
            .await
            .expect("read fixture request");
        assert!(request_bytes > 0, "fixture request was empty");
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write fixture headers");
        let chunk = vec![b' '; 64 * 1024];
        for _ in 0..=(super::MAX_INTEGRATION_RESPONSE_BYTES / chunk.len()) {
            stream
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await
                .expect("write chunk length");
            stream
                .write_all(&chunk)
                .await
                .expect("write oversized chunk");
            stream.write_all(b"\r\n").await.expect("finish chunk");
        }
        stream
            .write_all(b"0\r\n\r\n")
            .await
            .expect("finish response");
    });

    let response = reqwest::get(format!("http://{address}/oversized"))
        .await
        .expect("receive fixture headers");
    let error = super::read_bounded_integration_json(response, "fixture")
        .await
        .expect_err("chunked oversized response must be rejected");
    assert!(error.contains("response exceeds"), "{error}");
    server.await.expect("fixture server task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn source_provider_reader_rejects_declared_and_chunked_oversized_responses() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    for chunked in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind source provider fixture");
        let address = listener
            .local_addr()
            .expect("source provider fixture address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept fixture request");
            let mut request = [0_u8; 1024];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            if chunked {
                stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
                let chunk = vec![b'x'; 64 * 1024];
                for _ in 0..=(super::MAX_SOURCE_PROVIDER_RESPONSE_BYTES / chunk.len()) {
                    stream
                        .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                        .await
                        .unwrap();
                    stream.write_all(&chunk).await.unwrap();
                    stream.write_all(b"\r\n").await.unwrap();
                }
                let _ = stream.write_all(b"0\r\n\r\n").await;
            } else {
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            super::MAX_SOURCE_PROVIDER_RESPONSE_BYTES + 1
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        let response = reqwest::get(format!("http://{address}/oversized"))
            .await
            .expect("receive source provider fixture headers");
        let error = super::read_bounded_source_provider_bytes(response, "provider")
            .await
            .expect_err("oversized source provider response must fail");
        assert!(error.contains("response exceeds"), "{error}");
        server.await.expect("source provider fixture task");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn integration_ssrf_filter_blocks_special_use_ip_ranges() {
    let error = super::validate_integration_base_url("https://operator:secret@example.com/lidarr")
        .err()
        .expect("credential-bearing integration URL should be rejected");
    assert_eq!(
        error,
        "integration URL must not contain embedded credentials"
    );
    assert!(!error.contains("operator"));
    assert!(!error.contains("secret"));

    for url in [
        "https://example.com/lidarr?api_key=secret",
        "https://example.com/lidarr#ignored-api-path",
    ] {
        assert_eq!(
            super::validate_integration_base_url(url)
                .err()
                .expect("non-base integration URL should be rejected"),
            "integration URL must not contain a query or fragment"
        );
    }

    for address in [
        "100.64.0.1",
        "192.0.0.8",
        "192.31.196.1",
        "192.52.193.1",
        "192.88.99.1",
        "198.18.0.1",
    ] {
        let ip = address.parse::<std::net::IpAddr>().expect("fixture IP");
        assert!(super::is_blocked_integration_ip(ip), "accepted {address}");
    }
    for address in [
        "ff02::1",
        "2001:db8::1",
        "2002:c0a8:101::1",
        "2001:0000:4136:e378::1",
        "fec0::1",
        "64:ff9b::7f00:1",
        "64:ff9b:1::1",
        "100::1",
        "2001:2::1",
        "2001:10::1",
        "2001:20::1",
        "3fff::1",
    ] {
        let ip = address.parse::<std::net::IpAddr>().expect("fixture IP");
        assert!(super::is_blocked_integration_ip(ip), "accepted {address}");
    }
    assert!(!super::is_blocked_integration_ip(
        "2606:4700:4700::1111"
            .parse::<std::net::IpAddr>()
            .expect("global fixture IP")
    ));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
    let trusted_env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let trusted_config = super::AppConfig::from_layers(None, FileConfig::default(), &trusted_env)
        .expect("trusted proxy config");
    let untrusted_config =
        super::AppConfig::from_layers(None, FileConfig::default(), &MapEnv::default())
            .expect("default config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = super::http_server::HttpHeaders {
        x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
        ..Default::default()
    };

    let trusted_addr = super::rate_limit_remote_addr(&trusted_config, proxy, &headers)
        .expect("trusted forwarded address");
    assert_eq!(
        trusted_addr.ip(),
        "198.51.100.24".parse::<IpAddr>().unwrap()
    );

    let untrusted_addr = super::rate_limit_remote_addr(&untrusted_config, proxy, &headers)
        .expect("raw peer address");
    assert_eq!(untrusted_addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn authenticated_rate_limit_key_uses_verified_credential_identity() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "route-token")
            .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true"),
    )
    .expect("cookie auth config");
    let cookie = Some("slskr.session=route-token");
    let first = super::authenticated_rate_limit_user_key(&config, None, cookie, None)
        .expect("cookie-authenticated key");
    let second =
        super::authenticated_rate_limit_user_key(&config, Some("Bearer route-token"), cookie, None)
            .expect("matching credentials key");

    assert_eq!(first, second);
    assert_eq!(first, super::rate_limit_user_key("route-token"));
    let key = first;
    assert!(!key.contains("route-token"));
    assert!(super::authenticated_rate_limit_user_key(
        &config,
        Some("Bearer attacker-controlled"),
        cookie,
        None,
    )
    .is_none());
    assert!(super::authenticated_rate_limit_user_key(
        &config,
        Some("Basic route-token"),
        cookie,
        None,
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn authenticated_api_key_rate_limit_key_honors_cidr_from_request_peer() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKD_API_KEYS_JSON",
                r#"{"operator":{"key":"0123456789abcdef","role":"readonly","cidr":"127.0.0.1/32"}}"#,
            ),
    )
    .expect("API key config");
    let key = super::authenticated_rate_limit_user_key(
        &config,
        Some("ApiKey 0123456789abcdef"),
        None,
        Some("127.0.0.1:8080".parse().unwrap()),
    )
    .expect("CIDR-authorized API key must bypass the anonymous partition");
    assert_eq!(key, super::rate_limit_user_key("0123456789abcdef"));
    assert!(super::authenticated_rate_limit_user_key(
        &config,
        Some("ApiKey 0123456789abcdef"),
        None,
        Some("192.0.2.20:8080".parse().unwrap()),
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn frozen_controller_rate_limit_policies_match_path_and_auth_precedence() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_API_PERMIT_LIMIT", "17")
            .with("SLSKD_WEB_API_WINDOW_SECONDS", "0")
            .with("SLSKD_WEB_FEDERATION_PERMIT_LIMIT", "3")
            .with("SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT", "4"),
    )
    .expect("slskdN rate-limit config");
    let remote = Some("192.0.2.30:1234".parse().unwrap());

    let anonymous =
        super::controller_rate_limit_policy(&config, "GET", "/API/v0/searches", None, remote)
            .unwrap();
    assert_eq!(anonymous.partition, "api");
    assert_eq!(anonymous.max_requests, 17);
    assert_eq!(anonymous.window_seconds, 60);
    assert!(super::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/searches",
        Some("principal"),
        remote,
    )
    .is_none());
    assert!(super::controller_rate_limit_policy(&config, "GET", "/", None, remote,).is_none());

    let mesh = super::controller_rate_limit_policy(
        &config,
        "GET",
        "/MESH/gateway",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!((mesh.partition.as_str(), mesh.max_requests), ("mesh", 4));
    let inbox = super::controller_rate_limit_policy(
        &config,
        "POST",
        "/actors/alice/INBOX",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!((inbox.partition.as_str(), inbox.max_requests), ("fed", 3));
    let event = super::controller_rate_limit_policy(
        &config,
        "POST",
        "/api/v0/events/inject",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!(event.max_requests, 10);
    let warm = super::controller_rate_limit_policy(
        &config,
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!(warm.partition, "warm-cache:principal");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_profile_has_no_native_global_rate_limiter() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_WEB_RATE_LIMITING", "true"),
    )
    .expect("slskd config");
    assert!(super::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/searches",
        None,
        Some("192.0.2.31:1234".parse().unwrap()),
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_request_body_limit_validation_and_projection_match_frozen_shape() {
    for invalid in [
        serde_json::json!({"web": {"max_request_body_size": 0}}),
        serde_json::json!({"web": {"max_request_body_size": -1}}),
        serde_json::json!({"web": {"max_request_body_size": 2147483648_i64}}),
        serde_json::json!({"web": {"max_request_body_size": []}}),
        serde_json::json!({"web": []}),
    ] {
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &invalid,
                super::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }
    for valid in [
        serde_json::json!({"web": {"max_request_body_size": null}}),
        serde_json::json!({"web": {"max_request_body_size": 1}}),
        serde_json::json!({"web": {"max_request_body_size": "2147483647"}}),
    ] {
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &valid,
                super::ControllerProfile::Native,
            ),
            None
        );
    }

    let projected = super::controller_yaml_api_projection(serde_json::json!({
        "web": {"max_request_body_size": "7340032"}
    }));
    assert_eq!(projected["web"]["maxRequestBodySize"], 7 * 1024 * 1024);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_cors_validation_and_projection_match_frozen_shape() {
    let valid = serde_json::json!({
        "diagnostics": {
            "allow_memory_dump": "true",
            "allow_remote_dump": false
        },
        "web": {
            "enforce_security": "true",
            "allow_remote_no_auth": "false",
            "authentication": {
                "disabled": "true",
                "passthrough": {"allowed_cidrs": 123}
            },
            "cors": {
                "enabled": "true",
                "allow_credentials": false,
                "allowed_origins": ["https://allowed.example"],
                "allowed_headers": ["X-Custom"],
                "allowed_methods": null
            }
        }
    });
    assert_eq!(
        super::controller_yaml_target_validation_error(&valid, super::ControllerProfile::Native,),
        None
    );
    for invalid in [
        serde_json::json!({"web": {"cors": []}}),
        serde_json::json!({"web": {"cors": {"enabled": "yes"}}}),
        serde_json::json!({"web": {"cors": {"allow_credentials": 1}}}),
        serde_json::json!({"web": {"cors": {"allowed_origins": "*"}}}),
        serde_json::json!({"web": {"cors": {"allowed_methods": {}}}}),
        serde_json::json!({"web": {"enforce_security": "yes"}}),
        serde_json::json!({"web": {"allow_remote_no_auth": 1}}),
        serde_json::json!({"web": {"authentication": []}}),
        serde_json::json!({"web": {"authentication": {"disabled": "yes"}}}),
        serde_json::json!({"web": {"authentication": {"passthrough": []}}}),
        serde_json::json!({"web": {"authentication": {"passthrough": {"allowed_cidrs": []}}}}),
        serde_json::json!({"diagnostics": []}),
        serde_json::json!({"diagnostics": {"allow_memory_dump": "yes"}}),
        serde_json::json!({"diagnostics": {"allow_remote_dump": 1}}),
    ] {
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &invalid,
                super::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }

    let projected = super::controller_yaml_api_projection(valid);
    assert_eq!(projected["diagnostics"]["allowMemoryDump"], true);
    assert_eq!(projected["diagnostics"]["allowRemoteDump"], false);
    assert_eq!(projected["web"]["enforceSecurity"], true);
    assert_eq!(projected["web"]["allowRemoteNoAuth"], false);
    assert_eq!(projected["web"]["authentication"]["disabled"], true);
    assert_eq!(
        projected["web"]["authentication"]["passthrough"]["allowedCidrs"],
        "123"
    );
    assert_eq!(projected["web"]["cors"]["enabled"], true);
    assert_eq!(projected["web"]["cors"]["allowCredentials"], false);
    assert_eq!(
        projected["web"]["cors"]["allowedOrigins"],
        serde_json::json!(["https://allowed.example"])
    );
    assert!(projected["web"]["cors"].get("allowedMethods").is_none());

    let scalar_array = serde_json::json!({
        "web": {"cors": {"allowed_headers": ["X-Good", 1, true]}}
    });
    assert_eq!(
        super::controller_yaml_target_validation_error(
            &scalar_array,
            super::ControllerProfile::Native,
        ),
        None
    );
    let scalar_array = super::controller_yaml_api_projection(scalar_array);
    assert_eq!(
        scalar_array["web"]["cors"]["allowedHeaders"],
        serde_json::json!(["X-Good", "1", "true"])
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_rate_limit_validation_projection_and_nonpositive_runtime_match_frozen() {
    let valid = serde_json::json!({
        "web": {
            "rate_limiting": {
                "enabled": "true",
                "api_permit_limit": 0,
                "api_window_seconds": -1,
                "federation_permit_limit": "-2",
                "federation_window_seconds": null,
                "mesh_gateway_permit_limit": 2147483647_i64,
                "mesh_gateway_window_seconds": -2147483648_i64
            }
        }
    });
    assert_eq!(
        super::controller_yaml_target_validation_error(&valid, super::ControllerProfile::Native,),
        None
    );
    for invalid in [
        serde_json::json!({"web": {"rate_limiting": []}}),
        serde_json::json!({"web": {"rate_limiting": {"enabled": "nope"}}}),
        serde_json::json!({"web": {"rate_limiting": {"api_permit_limit": 1.5}}}),
        serde_json::json!({"web": {"rate_limiting": {"api_window_seconds": 2147483648_i64}}}),
        serde_json::json!({"web": {"rate_limiting": {"federation_permit_limit": []}}}),
    ] {
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &invalid,
                super::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }

    let projected = super::controller_yaml_api_projection(valid);
    assert_eq!(projected["web"]["rateLimiting"]["enabled"], true);
    assert_eq!(projected["web"]["rateLimiting"]["apiPermitLimit"], 0);
    assert_eq!(
        projected["web"]["rateLimiting"]["federationPermitLimit"],
        -2
    );

    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_WEB_API_PERMIT_LIMIT", "0"),
    )
    .expect("frozen non-positive permit value remains startup-valid");
    let policy = super::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/options",
        None,
        Some("192.0.2.33:1234".parse().unwrap()),
    )
    .unwrap();
    assert_eq!(policy.max_requests, 0);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_rate_limit_watch_projects_current_values_and_requests_restart() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let yaml = "web:\n  max_request_body_size: 7340032\n  cors:\n    enabled: true\n    allow_credentials: true\n    allowed_origins: [https://allowed.example]\n    allowed_headers: [X-Custom]\n    allowed_methods: [GET, POST]\n  rate_limiting:\n    enabled: true\n    api_permit_limit: 9\n    api_window_seconds: -1\n    federation_permit_limit: 8\n    federation_window_seconds: 7\n    mesh_gateway_permit_limit: 6\n    mesh_gateway_window_seconds: 5\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(
        state.config.controller_web_rate_limiting.api_permit_limit,
        200
    );
    assert_eq!(
        state.config.controller_web_max_request_body_size,
        10 * 1024 * 1024
    );
    let overlay = state.options_overlay.read().await;
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(
        options["web"]["rateLimiting"],
        serde_json::json!({
            "enabled": true,
            "apiPermitLimit": 9,
            "apiWindowSeconds": -1,
            "federationPermitLimit": 8,
            "federationWindowSeconds": 7,
            "meshGatewayPermitLimit": 6,
            "meshGatewayWindowSeconds": 5,
        })
    );
    assert_eq!(options["web"]["maxRequestBodySize"], 7 * 1024 * 1024);
    assert_eq!(
        options["web"]["cors"],
        serde_json::json!({
            "enabled": true,
            "allowCredentials": true,
            "allowedOrigins": ["https://allowed.example"],
            "allowedHeaders": ["X-Custom"],
            "allowedMethods": ["GET", "POST"],
        })
    );
    assert!(!state.config.controller_web_cors.enabled);

    let policy = super::controller_rate_limit_policy(
        &state.config,
        "GET",
        "/api/v0/session",
        None,
        Some("192.0.2.32:1234".parse().unwrap()),
    )
    .unwrap();
    assert_eq!(policy.max_requests, 200);
    assert_eq!(policy.window_seconds, 60);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn cookie_auth_rejects_noncanonical_token_encodings() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "token-�")
            .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true"),
    )
    .expect("cookie auth config");

    assert!(super::is_authorized(
        &config,
        None,
        Some("slskr.session=token-%EF%BF%BD")
    ));
    for cookie in [
        "slskr.session=token-%FF",
        "slskr.session=token-%",
        "slskr.session=token-%G0",
        "slskr.session=token-%FF; slskr.session=token-%EF%BF%BD",
        "slskr.session=token-%EF%BF%BD; slskr.session=token-%EF%BF%BD",
        "slskr.session=wrong; slskr.session=token-%EF%BF%BD",
    ] {
        assert!(
            !super::is_authorized(&config, None, Some(cookie)),
            "{cookie}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "::1/128");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("[::1]:5000".parse::<SocketAddr>().unwrap());
    let headers = super::http_server::HttpHeaders {
        forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
        ..Default::default()
    };

    let addr =
        super::rate_limit_remote_addr(&config, proxy, &headers).expect("trusted forwarded address");
    assert_eq!(addr.ip(), "2001:db8::42".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn forwarded_ip_parser_rejects_malformed_authorities() {
    for malformed in [
        "\"\"198.51.100.24\"\"",
        "\"198.51.100.24",
        "198.51.100.24\"",
        "[2001:db8::42]garbage",
        "[2001:db8::42]:garbage",
        "[2001:db8::42]:65536",
        "[198.51.100.24]:80",
        "198.51.100.24:65536",
        "198.51.100.24:80:90",
    ] {
        assert_eq!(
            super::parse_forwarded_ip_token(malformed),
            None,
            "accepted {malformed}"
        );
    }
    assert_eq!(
        super::parse_forwarded_ip_token("\"[2001:db8::42]:443\""),
        Some("2001:db8::42".parse::<IpAddr>().unwrap())
    );
    assert_eq!(
        super::parse_forwarded_ip_token("198.51.100.24:443"),
        Some("198.51.100.24".parse::<IpAddr>().unwrap())
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn forwarded_elements_require_one_valid_for_parameter() {
    let expected = "198.51.100.24".parse::<IpAddr>().unwrap();
    assert_eq!(
        super::parse_forwarded_element_ip("proto=https; for=198.51.100.24; by=10.0.0.2"),
        Some(expected)
    );
    for malformed in [
        "proto=https",
        "for=unknown;for=198.51.100.24",
        "for=198.51.100.24;for=198.51.100.24",
        "for=198.51.100.24;for=203.0.113.9",
        "for=198.51.100.24;broken",
    ] {
        assert_eq!(
            super::parse_forwarded_element_ip(malformed),
            None,
            "accepted {malformed:?}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn trusted_proxy_rate_limit_addr_rejects_spoofed_leftmost_hop() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32, 10.0.0.0/8");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = super::http_server::HttpHeaders {
        x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
        ..Default::default()
    };

    let addr =
        super::rate_limit_remote_addr(&config, proxy, &headers).expect("forwarded client address");
    assert_eq!(
        addr.ip(),
        "198.51.100.24".parse::<IpAddr>().unwrap(),
        "the first untrusted hop from the proxy boundary is the client"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn trusted_proxy_rate_limit_addr_fails_closed_on_malformed_chain() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = super::http_server::HttpHeaders {
        x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
        ..Default::default()
    };

    let addr =
        super::rate_limit_remote_addr(&config, proxy, &headers).expect("socket peer fallback");
    assert_eq!(addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = super::http_server::HttpHeaders {
        forwarded: Some("for=unknown".to_owned()),
        x_forwarded_for: Some("203.0.113.99".to_owned()),
        ..Default::default()
    };

    let addr =
        super::rate_limit_remote_addr(&config, proxy, &headers).expect("socket peer fallback");
    assert_eq!(addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

fn test_state_with_env_parts(
    extra_env: MapEnv,
    search_store: super::SearchStore,
    db: Option<super::persistence::DatabaseManager>,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    test_state_with_env_parts_full(
        extra_env,
        search_store,
        super::MessageStore::new(),
        super::RoomStore::new(),
        db,
    )
}

fn test_state_with_env_parts_full(
    extra_env: MapEnv,
    search_store: super::SearchStore,
    message_store: super::MessageStore,
    room_store: super::RoomStore,
    db: Option<super::persistence::DatabaseManager>,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let state_dir =
        std::env::temp_dir().join(format!("slskr-route-test-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&state_dir).unwrap();
    let mut env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTH_DISABLED", "true")
        .with("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
        .with("SLSK_USERNAME", "tester")
        .with("SLSK_PASSWORD", "secret")
        // Keep route tests hermetic: individual MusicBrainz tests replace
        // this endpoint with an in-process fixture when they exercise the
        // integration itself.
        .with("SLSKD_MUSICBRAINZ_BASE_URL", "http://127.0.0.1:9")
        .with("SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS", "0.05")
        .with("SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS", "1");
    env.values.extend(extra_env.values);
    let controller_cli_environment = env.values.clone();
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("test config");
    let share_index = super::build_share_index(&config);
    let share_lifecycle = super::ShareLifecycleState::from_snapshot(&share_index);
    let (sender, receiver) = mpsc::channel(8);
    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
    let rate_limiter = super::rate_limit::RateLimiter::new(super::rate_limit::RateLimitConfig {
        max_requests_anonymous: 1000,
        max_requests_authenticated: 5000,
        window_seconds: 60,
        enabled: true,
    });
    let distributed_runtime = super::DistributedRuntime::new(config.username.as_deref());
    let (distributed_persistence_snapshots, distributed_persistence_receiver) =
        tokio::sync::watch::channel(distributed_runtime.persistence_snapshot());
    let (distributed_persistence_status_sender, distributed_persistence_status) =
        tokio::sync::watch::channel(super::DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });

    let state = Arc::new(super::AppState {
        controller_version: std::sync::RwLock::new(super::ControllerVersionState::initial()),
        controller_cli_environment,
        log_level: RwLock::new(super::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(config.credentials()),
        controller_web_auth_username: std::sync::RwLock::new(
            config.controller_web_auth_username.clone(),
        ),
        controller_web_auth_password: std::sync::RwLock::new(
            config.controller_web_auth_password.clone(),
        ),
        controller_web_jwt_key_current: std::sync::RwLock::new(
            config.controller_web_jwt_key.clone(),
        ),
        session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
        server_address: std::sync::RwLock::new(config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(super::ListenerSnapshot::new(&config)),
        distributed_network: RwLock::new(distributed_runtime),
        distributed_persistence_snapshots,
        distributed_persistence_status,
        soulseek_distributed_settings: RwLock::new(config.soulseek_distributed),
        shares: RwLock::new(share_index),
        share_settings: RwLock::new(config.share_settings.clone()),
        share_index_persistence_lock: tokio::sync::Mutex::new(()),
        share_settings_generation: std::sync::atomic::AtomicU64::new(0),
        core_workflow_settings: RwLock::new(config.core_workflow.clone()),
        advanced_networking: RwLock::new(config.advanced_networking.clone()),
        media_services: RwLock::new(config.media_services.clone()),
        share_lifecycle: RwLock::new(share_lifecycle),
        downloads_dir: std::sync::RwLock::new(config.downloads_dir.clone()),
        incomplete_dir: std::sync::RwLock::new(config.incomplete_dir.clone()),
        download_completed_path_template: std::sync::RwLock::new(
            config.download_completed_path_template.clone(),
        ),
        remote_file_management: std::sync::RwLock::new(config.remote_file_management),
        remote_configuration: std::sync::RwLock::new(config.remote_configuration),
        controller_no_config_watch: std::sync::RwLock::new(config.controller_no_config_watch),
        controller_case_sensitive_regex: std::sync::RwLock::new(
            config.controller_case_sensitive_regex,
        ),
        user_info_description: std::sync::RwLock::new(config.user_info_description.clone()),
        user_info_picture: std::sync::RwLock::new(config.user_info_picture.clone()),
        controller_options_validation_error: std::sync::RwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: std::sync::RwLock::new(config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(config.obfuscated_advertised_port),
        searches: RwLock::new(search_store),
        users: RwLock::new(super::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(super::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability signing key"),
        content_discovery: RwLock::new(super::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(super::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(super::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(super::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(message_store),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(super::ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_profile,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            super::compile_controller_regexes(
                &config.controller_search_request_filters,
                config.controller_case_sensitive_regex,
                config.controller_profile,
            )
            .unwrap(),
        ),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(super::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(super::LidarrSyncRuntimeState::new(
            &config.integrations.lidarr,
        )),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(config.transfer_auto_retry.clone()),
        transfer_upload_settings: RwLock::new(config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(super::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            super::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(room_store),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(super::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(super::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(super::pod_channels::PodChannelStore::empty(
            &config.state_dir,
        )),
        pods: RwLock::new(super::pods::PodStore::empty(&config.state_dir)),
        port_forwarding: super::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(super::TransferQueue::new(&config)),
        events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
        event_tx: event_tx.clone(),
        webhooks: Arc::new(RwLock::new(super::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(super::Semaphore::new(super::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(super::Semaphore::new(super::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(super::Semaphore::new(super::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(super::Semaphore::new(
            config.core_workflow.incoming_search.concurrency,
        )),
        incoming_search_queue_depth: super::AtomicUsize::new(0),
        download_requests: Arc::new(super::Semaphore::new(2)),
        download_batch_requests: Arc::new(super::Semaphore::new(1)),
        websocket_connections: Arc::new(super::Semaphore::new(super::MAX_WEBSOCKET_CONNECTIONS)),
        external_visualizer_processes: Arc::new(super::Semaphore::new(
            super::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        songid_run_slots: Arc::new(super::Semaphore::new(
            config.media_services.song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(super::CollectionStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(super::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(super::ContactStore::new()),
        sharegroups: RwLock::new(super::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(super::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(super::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(super::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(super::RelayState::new()),
        runtime: RwLock::new(super::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(super::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(super::BackfillState::default()),
        backfill_connections: Arc::new(super::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(super::SecurityState::new()),
        share_grants: RwLock::new(super::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(super::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(super::IncomingShareStore::default()),
        library: RwLock::new(super::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(super::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(super::SourceDiscoveryState::default()),
        destinations: RwLock::new(super::DestinationStore::new()),
        db,
        config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: super::ManagedTaskRegistry::default(),
        rate_limiter,
        soulseek_safety: super::rate_limit::SoulseekSafetyLimiter::new(
            super::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(super::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(super::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(super::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(super::multisource::SwarmStore::default())),
        controller_features: super::ControllerFeatureStore::new(
            super::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(super::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(super::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(super::RevokedJwtStore::default()),
        login_attempts: RwLock::new(super::LoginAttemptStore::default()),
        pod_signature_stats: super::PodSignatureStats::default(),
        pod_verification_stats: super::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: super::PodCoreRuntimeStats::default(),
    });
    if tokio::runtime::Handle::try_current().is_ok() {
        state.spawn_managed_task(super::run_distributed_persistence_worker(
            state.db.clone(),
            distributed_persistence_receiver,
            distributed_persistence_status_sender,
        ));
    } else {
        drop(distributed_persistence_receiver);
        drop(distributed_persistence_status_sender);
    }
    (state, receiver)
}

async fn add_test_share(state: &Arc<super::AppState>, filename: &str, path: &Path, size: u64) {
    let mut shares = state.shares.write().await;
    shares
        .local_paths
        .insert(filename.to_owned(), path.to_path_buf());
    shares.entries.push(FileEntry {
        filename_encoding: Default::default(),
        extension_encoding: Default::default(),
        code: 1,
        filename: filename.to_owned(),
        size,
        extension: super::extension_for(filename),
        attributes: Vec::new(),
    });
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn parse_route_reads_method_and_path() {
    assert_eq!(
        parse_route("POST /api/session/connect HTTP/1.1\r\nhost: localhost\r\n\r\n"),
        ("POST", "/api/session/connect")
    );
    assert_eq!(
        split_request_target("/api/v0/shares/catalog?q=test"),
        ("/api/v0/shares/catalog", Some("q=test"))
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn versioned_api_paths_map_to_current_handlers() {
    assert_eq!(normalize_api_path("/api/v0/health"), "/api/health");
    assert_eq!(normalize_api_path("/api/v0/metrics"), "/api/metrics");
    assert_eq!(normalize_api_path("/api/v0/telemetry"), "/api/telemetry");
    assert_eq!(
        normalize_api_path("/api/v0/capabilities/negotiate"),
        "/api/capabilities/negotiate"
    );
    assert_eq!(
        normalize_api_path("/api/v0/session/connect"),
        "/api/session/connect"
    );
    assert_eq!(normalize_api_path("/api/v0/dht/peers"), "/api/dht/peers");
    assert_eq!(normalize_api_path("/api/custom"), "/api/custom");
    assert_eq!(normalize_api_path("/api/info"), "/api/application");
    assert_eq!(
        normalize_api_path("/api/v0/portforwarding/available-ports"),
        "/api/port-forwarding/available-ports"
    );
    assert_eq!(
        normalize_api_path("/api/v0/portforwarding/start"),
        "/api/port-forwarding/start"
    );
    // Matches the oracle's real, distinct GetMeshPeers endpoint --
    // previously collapsed into the same (wrong) handler as
    // GET /api/capabilities/peers.
    assert_eq!(
        normalize_api_path("/api/v0/capabilities/mesh-peers"),
        "/api/v0/capabilities/mesh-peers"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn versioned_public_routes_share_auth_policy_with_canonical_routes() {
    let env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");

    for path in ["/api/v0/health", "/api/v0/version", "/api/v0/capabilities"] {
        assert!(!super::route_requires_auth(&config, path), "{path}");
    }
    assert!(super::route_requires_auth(&config, "/api/v0/config"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn read_only_api_routes_return_contract_shapes() {
    let (state, _receiver) = test_state();

    let cases = [
        ("/api/v0/health", "\"status\":\"ok\""),
        ("/api/v0/version", "\"name\":\"slskr\""),
        (
            "/api/v0/capabilities",
            "\"version\":\"slskdn/1.0.0+dht+mesh+swarm\"",
        ),
        ("/api/v0/config", "\"credentials_configured\":true"),
        ("/api/v0/stats", "\"session\":"),
        ("/api/v0/telemetry", "\"health\":"),
        ("/api/v0/events", "[]"),
        ("/api/v0/events/records", "\"entries\":"),
        ("/api/v0/logs", "[]"),
        ("/api/v0/session/enabled", "false"),
        ("/api/v0/listeners", "\"regular_accepts\":0"),
        ("/api/v0/users", "\"count\":0"),
        ("/api/v0/rooms", "\"count\":0"),
        ("/api/v0/shares", "\"files\":1"),
        ("/api/v0/shares/catalog", "\"total_bytes\":42"),
        ("/api/v0/searches", "[]"),
        ("/api/v0/searches/records", "\"count\":0"),
        ("/api/v0/transfers", "[]"),
        ("/api/v0/transfers/stats", "\"total\":0"),
    ];

    for (path, expected_body) in cases {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "200 OK", "{path}");
        assert_eq!(
            response.content_type,
            if path == "/api/v0/shares" {
                "application/json; charset=utf-8"
            } else {
                "application/json"
            },
            "{path}"
        );
        assert!(
            response.body.contains(expected_body),
            "{path}: {}",
            response.body
        );
        assert!(
            !response.body.contains("test-password")
                && !response.body.contains("api-token")
                && !response.body.contains("client-secret"),
            "{path}: {}",
            response.body
        );
    }
    let session_check = super::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("versioned session check");
    assert_eq!(session_check.status, "200 OK");
    assert_eq!(session_check.content_type, "application/json");
    assert!(
        session_check.body.contains("\"state\":"),
        "{}",
        session_check.body
    );
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_bounded_activity_and_network_polling_routes_project_local_state(
) {
    let (state, _receiver) = test_state();

    let message_id = {
        let mut messages = state.messages.write().await;
        messages
            .add("peer-unread".to_owned(), "inbound", "hello".to_owned())
            .id
    };
    let unread = super::route_http_request(
        "GET",
        "/api/v0/conversations/activity/unacknowledged",
        None,
        "",
        &state,
    )
    .await
    .expect("unacknowledged activity");
    assert_eq!(unread.status, "200 OK");
    assert!(unread.content_type.contains("application/json"));
    assert_eq!(unread.body, "true");
    state.messages.write().await.ack(message_id);
    let acknowledged = super::route_http_request(
        "GET",
        "/api/v0/conversations/activity/unacknowledged",
        None,
        "",
        &state,
    )
    .await
    .expect("acknowledged activity");
    assert_eq!(acknowledged.status, "200 OK");
    assert!(acknowledged.content_type.contains("application/json"));
    assert_eq!(acknowledged.body, "false");

    let local_username = state
        .session
        .read()
        .await
        .username
        .clone()
        .unwrap_or_else(|| "local".to_owned());
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("active-room".to_owned()).expect("room capacity");
        rooms
            .add_message("active-room", local_username, "outbound".to_owned())
            .expect("joined room");
        rooms
            .add_message("active-room", "peer-room".to_owned(), "inbound".to_owned())
            .expect("joined room");
        rooms.join("left-room".to_owned()).expect("room capacity");
        rooms
            .add_message("left-room", "peer-room".to_owned(), "ignored".to_owned())
            .expect("joined room");
        rooms.leave("left-room").expect("existing room");
    }
    let activity = super::route_http_request("GET", "/api/v0/rooms/activity", None, "", &state)
        .await
        .expect("room activity");
    assert_eq!(activity.status, "200 OK");
    assert!(activity.content_type.contains("application/json"));
    let activity_json = serde_json::from_str::<serde_json::Value>(&activity.body).unwrap();
    assert!(activity_json["active-room"].as_u64().unwrap_or_default() > 0);
    assert!(activity_json.get("left-room").is_none());

    let network = super::route_http_request(
        "GET",
        "/api/v0/network/stats?includePeers=false",
        None,
        "",
        &state,
    )
    .await
    .expect("network stats");
    assert_eq!(network.status, "200 OK");
    assert!(network.content_type.contains("application/json"));
    let network_json = serde_json::from_str::<serde_json::Value>(&network.body).unwrap();
    for key in [
        "backfill",
        "capabilitiesJson",
        "capabilitiesVersion",
        "dht",
        "discoveredPeers",
        "hashDb",
        "mesh",
        "meshPeers",
        "swarmJobs",
        "transport",
    ] {
        assert!(network_json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(network_json["discoveredPeers"], serde_json::json!([]));
    assert_eq!(network_json["meshPeers"], serde_json::json!([]));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/conversations/activity/unacknowledged",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/activity",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("activity_and_network_populated.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_network_stats_edge_contracts() {
    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native");

    let (malformed_state, _receiver) = test_state_with_env(target_env());
    let malformed = super::route_http_request(
        "GET",
        "/api/v0/network/stats?includePeers=not-a-boolean",
        None,
        "",
        &malformed_state,
    )
    .await
    .expect("network stats malformed query");
    assert_eq!(malformed.status, "400 Bad Request");
    assert!(malformed.body.contains("boolean"), "{}", malformed.body);

    let (empty_state, _receiver) = test_state_with_env(target_env());
    let empty = super::route_http_request("GET", "/api/v0/network/stats", None, "", &empty_state)
        .await
        .expect("network stats empty state");
    assert_eq!(empty.status, "200 OK");
    let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    for key in [
        "backfill",
        "capabilitiesJson",
        "capabilitiesVersion",
        "dht",
        "discoveredPeers",
        "hashDb",
        "mesh",
        "meshPeers",
        "swarmJobs",
        "transport",
    ] {
        assert!(empty_json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(empty_json["discoveredPeers"], serde_json::json!([]));
    assert_eq!(empty_json["meshPeers"], serde_json::json!([]));
    assert_eq!(empty_json["swarmJobs"], serde_json::json!([]));

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("network stats runtime db");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
    db.close_for_test().await;
    let runtime =
        super::route_http_request("GET", "/api/v0/network/stats", None, "", &runtime_state)
            .await
            .expect("network stats closed-database state");
    assert_eq!(runtime.status, "200 OK");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert!(runtime_json.get("dht").is_some());
    assert!(runtime_json.get("transport").is_some());

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "malformed-path-query-or-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "missing-empty-or-conflict-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("network_stats_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_read_projection_runtime_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let federation_env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "runtime.example")
            .with("FEDERATION_BASE_URL", "https://runtime.example/")
    };

    let (logs_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let malformed_logs =
        super::route_http_request("GET", "/api/v0/logs/malformed", None, "", &logs_state)
            .await
            .expect("malformed logs path");
    record!(
        "/api/v0/logs",
        "malformed-path-query-or-body",
        malformed_logs.status == "404 Not Found"
    );
    let empty_logs = super::route_http_request("GET", "/api/v0/logs", None, "", &logs_state)
        .await
        .expect("empty logs projection");
    let empty_logs_json =
        serde_json::from_str::<serde_json::Value>(&empty_logs.body).unwrap_or_default();
    record!(
        "/api/v0/logs",
        "missing-empty-or-conflict-state",
        empty_logs.status == "200 OK" && empty_logs_json.as_array().is_some()
    );
    let logs_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("logs runtime database");
    let (logs_runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(logs_db.clone()),
    );
    logs_db.close_for_test().await;
    let runtime_logs =
        super::route_http_request("GET", "/api/v0/logs", None, "", &logs_runtime_state)
            .await
            .expect("logs closed-database projection");
    record!(
        "/api/v0/logs",
        "runtime-failure-and-timeout",
        runtime_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_logs.body)
                .is_ok_and(|value| value.is_array())
    );

    let (federation_state, _receiver) = test_state_with_env(federation_env());
    let malformed_federation = super::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics/malformed",
        None,
        "",
        &federation_state,
    )
    .await
    .expect("malformed federation diagnostics path");
    record!(
        "/api/v0/federation/diagnostics",
        "malformed-path-query-or-body",
        malformed_federation.status == "404 Not Found"
    );
    let empty_federation = super::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics",
        None,
        "",
        &federation_state,
    )
    .await
    .expect("empty federation diagnostics projection");
    let empty_federation_json =
        serde_json::from_str::<serde_json::Value>(&empty_federation.body).unwrap_or_default();
    record!(
        "/api/v0/federation/diagnostics",
        "missing-empty-or-conflict-state",
        empty_federation.status == "200 OK"
            && empty_federation_json["federation"].is_object()
            && empty_federation_json["warnings"].is_array()
    );
    let federation_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("federation runtime database");
    let (federation_runtime_state, _receiver) = test_state_with_env_parts(
        federation_env(),
        super::SearchStore::new(),
        Some(federation_db.clone()),
    );
    federation_db.close_for_test().await;
    let runtime_federation = super::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics",
        None,
        "",
        &federation_runtime_state,
    )
    .await
    .expect("federation closed-database projection");
    let runtime_federation_json =
        serde_json::from_str::<serde_json::Value>(&runtime_federation.body).unwrap_or_default();
    record!(
        "/api/v0/federation/diagnostics",
        "runtime-failure-and-timeout",
        runtime_federation.status == "200 OK" && runtime_federation_json["federation"].is_object()
    );

    let webfinger_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("WebFinger runtime database");
    let (webfinger_runtime_state, _receiver) = test_state_with_env_parts(
        federation_env(),
        super::SearchStore::new(),
        Some(webfinger_db.clone()),
    );
    webfinger_db.close_for_test().await;
    let runtime_webfinger = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40runtime.example",
        None,
        "",
        &webfinger_runtime_state,
    )
    .await
    .expect("WebFinger closed-database projection");
    record!(
        "/.well-known/webfinger",
        "runtime-failure-and-timeout",
        runtime_webfinger.status == "200 OK"
            && runtime_webfinger.content_type == "application/jrd+json"
    );

    let collection_id = "00000000-0000-0000-0000-000000000001";
    let collections_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("collections runtime database");
    let (collections_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(collections_db.clone()),
    );
    collections_db.close_for_test().await;
    let runtime_collections =
        super::route_http_request("GET", "/api/v0/collections", None, "", &collections_state)
            .await
            .expect("collections closed-database list");
    record!(
        "/api/v0/collections",
        "runtime-failure-and-timeout",
        runtime_collections.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_collections.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_collection = super::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &collections_state,
    )
    .await
    .expect("collection closed-database detail");
    record!(
        "/api/v0/collections/{id}",
        "runtime-failure-and-timeout",
        runtime_collection.status == "404 Not Found"
    );
    let runtime_collection_items = super::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        "",
        &collections_state,
    )
    .await
    .expect("collection closed-database items");
    record!(
        "/api/v0/collections/{id}/items",
        "runtime-failure-and-timeout",
        runtime_collection_items.status == "404 Not Found"
    );

    let sharegroup_id = "00000000-0000-0000-0000-000000000002";
    let sharegroups_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("sharegroups runtime database");
    let (sharegroups_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(sharegroups_db.clone()),
    );
    sharegroups_db.close_for_test().await;
    let runtime_sharegroups =
        super::route_http_request("GET", "/api/v0/sharegroups", None, "", &sharegroups_state)
            .await
            .expect("sharegroups closed-database list");
    record!(
        "/api/v0/sharegroups",
        "runtime-failure-and-timeout",
        runtime_sharegroups.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_sharegroups.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_sharegroup = super::route_http_request(
        "GET",
        &format!("/api/v0/sharegroups/{sharegroup_id}"),
        None,
        "",
        &sharegroups_state,
    )
    .await
    .expect("sharegroup closed-database detail");
    record!(
        "/api/v0/sharegroups/{id}",
        "runtime-failure-and-timeout",
        runtime_sharegroup.status == "404 Not Found"
    );
    let runtime_sharegroup_members = super::route_http_request(
        "GET",
        &format!("/api/v0/sharegroups/{sharegroup_id}/members"),
        None,
        "",
        &sharegroups_state,
    )
    .await
    .expect("sharegroup closed-database members");
    record!(
        "/api/v0/sharegroups/{id}/members",
        "runtime-failure-and-timeout",
        runtime_sharegroup_members.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("native_read_projection_runtime_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api read projection mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_solid_status_edge_contracts() {
    let target = "slskdn";
    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());

    let malformed =
        super::route_http_request("GET", "/api/v0/solid/status/extra", None, "", &state)
            .await
            .expect("malformed Solid status path");
    assert_eq!(malformed.status, "404 Not Found");

    let empty = super::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
        .await
        .expect("empty Solid status");
    let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    assert_eq!(empty.status, "200 OK");
    assert_eq!(empty_json["enabled"], true);
    assert!(empty_json["clientId"].is_string());
    assert!(empty_json["redirectPath"].is_string());

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("Solid status runtime database");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
    db.close_for_test().await;
    let runtime =
        super::route_http_request("GET", "/api/v0/solid/status", None, "", &runtime_state)
            .await
            .expect("Solid status closed-database response");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert_eq!(runtime.status, "200 OK");
    assert_eq!(runtime_json["enabled"], true);
    assert!(runtime_json["clientId"].is_string());

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "malformed-path-query-or-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "missing-empty-or-conflict-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("solid_status_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_solid_resolution_runtime_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let target = "slskdn";
    async fn configure_failure(state: &Arc<super::AppState>) {
        let mut media = state.media_services.write().await;
        media.solid.allow_insecure_http = true;
        media.solid.allow_localhost_for_web_id = true;
        media.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
        media.solid.timeout = Duration::from_millis(50);
    }
    let body = r#"{"webId":"http://127.0.0.1:9/profile#me"}"#;

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("Solid resolution runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    configure_failure(&runtime_state).await;
    db.close_for_test().await;
    let runtime = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        body,
        &runtime_state,
    )
    .await
    .expect("Solid resolution runtime failure");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert_eq!(runtime.status, "500 Internal Server Error");
    assert_eq!(runtime_json["title"], "Failed to resolve WebID");

    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&restart_state).await;
    let restarted = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        body,
        &restart_state,
    )
    .await
    .expect("Solid resolution restart failure");
    assert_eq!(restarted.status, "500 Internal Server Error");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&restarted.body).unwrap()["title"],
        "Failed to resolve WebID"
    );

    let (concurrent_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&concurrent_state).await;
    let (first, second) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v0/solid/resolve-webid",
            None,
            body,
            &concurrent_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/solid/resolve-webid",
            None,
            body,
            &concurrent_state,
        )
    );
    assert_eq!(
        first.expect("first concurrent Solid failure").status,
        "500 Internal Server Error"
    );
    assert_eq!(
        second.expect("second concurrent Solid failure").status,
        "500 Internal Server Error"
    );

    let (mutation_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&mutation_state).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Solid profile fixture");
    let port = listener.local_addr().expect("Solid fixture address").port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://runtime-issuer.example/oidc>.\n"
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("Solid profile request");
        let mut request = [0_u8; 4096];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read Solid profile request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/turtle\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            profile.len(),
            profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write Solid profile response");
    });
    let resolved = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &mutation_state,
    )
    .await
    .expect("Solid resolution readback");
    server.await.expect("Solid profile fixture task");
    let resolved_json = serde_json::from_str::<serde_json::Value>(&resolved.body).unwrap();
    let mutation_pass = resolved.status == "200 OK"
        && resolved_json["webId"] == web_id
        && resolved_json["oidcIssuers"]
            == serde_json::json!(["https://runtime-issuer.example/oidc"]);
    assert!(mutation_pass, "{}", resolved.body);

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "restart-persistence-or-reset",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "concurrency-and-idempotency",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "mutation-side-effects-and-readback",
            "pass": mutation_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("solid_resolution_runtime_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn build_info_uses_app_version_not_protocol_version() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("GET", "/api/application/build", None, "", &state)
        .await
        .unwrap();
    assert_eq!(response.status, "200 OK");
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();

    assert_eq!(body["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        body["full"],
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION")
        )
    );
    assert_eq!(body["latestTag"], "");
    assert_eq!(
        body["protocol"]["major"],
        serde_json::json!(super::CLIENT_MAJOR_VERSION)
    );
    assert_eq!(
        body["protocol"]["minor"],
        serde_json::json!(super::CLIENT_MINOR_VERSION)
    );
    assert_ne!(
        body["current"],
        serde_json::json!(format!(
            "{}.{}.{}",
            super::CLIENT_NAME,
            super::CLIENT_MAJOR_VERSION,
            super::CLIENT_MINOR_VERSION
        ))
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_release_comparison_matches_frozen_native_rules() {
    use super::{is_newer_controller_release_available, normalize_controller_release_version};

    assert_eq!(
        normalize_controller_release_version(" refs/tags/V1.2.3+build.4 "),
        "1.2.3"
    );
    assert_eq!(
        normalize_controller_release_version("BUILD-DEV-20260717-slskdn.2"),
        "20260717-slskdn.2"
    );
    assert!(!is_newer_controller_release_available("1.2.3", "v1.2.3"));
    assert!(is_newer_controller_release_available("1.2.3", "1.2.4"));
    assert!(!is_newer_controller_release_available("1.2.4", "1.2.3"));
    assert!(is_newer_controller_release_available(
        "20260717-slskdn.1",
        "20260717-slskdn.2"
    ));
    assert!(!is_newer_controller_release_available(
        "20260718-slskdn.1",
        "20260717-slskdn.99"
    ));
    assert!(is_newer_controller_release_available("manual", "release"));
    assert!(!is_newer_controller_release_available("manual", ""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transient_credential_stores_refuse_bursts_at_live_capacity() {
    let mut expired_oauth = super::OAuthStateStore::with_max_records(1);
    let expired_state = expired_oauth
        .issue("spotify", "http://localhost/callback", 0)
        .unwrap();
    assert!(expired_oauth.consume("spotify", &expired_state).is_none());

    let mut oauth = super::OAuthStateStore::with_max_records(1);
    let state = oauth
        .issue("spotify", "http://localhost/callback", 600)
        .unwrap();
    assert!(oauth
        .issue("spotify", "http://localhost/callback", 600)
        .is_none());
    assert!(oauth.consume("spotify", &state).is_some());
    assert!(oauth
        .issue("spotify", "http://localhost/callback", 600)
        .is_some());

    let mut expired_tickets = super::PreviewStreamTicketStore::with_max_records(1);
    let (expired_ticket, _) = expired_tickets
        .issue(
            "peer",
            "test",
            "expired".to_owned(),
            "expired.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            0,
        )
        .unwrap();
    assert!(expired_tickets.get(&expired_ticket).is_none());

    let mut tickets = super::PreviewStreamTicketStore::with_max_records(1);
    assert!(tickets
        .issue(
            "peer",
            "test",
            "content-1".to_owned(),
            "song.flac".to_owned(),
            Some("alice".to_owned()),
            123,
            "audio/flac".to_owned(),
            120,
        )
        .is_some());
    assert!(tickets
        .issue(
            "peer",
            "test",
            "content-2".to_owned(),
            "other.flac".to_owned(),
            Some("bob".to_owned()),
            456,
            "audio/flac".to_owned(),
            120,
        )
        .is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_oauth_callback_requires_server_issued_state() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:7788"),
    );

    let authorize = super::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    assert_eq!(authorize.status, "200 OK");
    assert!(authorize
        .body
        .contains("127.0.0.1:7788/api/v0/integrations/spotify/callback"));

    let authorization: serde_json::Value = serde_json::from_str(&authorize.body).unwrap();
    let authorization_url = authorization["authorizationUrl"].as_str().unwrap();
    assert!(authorization_url.contains("code_challenge_method=S256"));
    assert!(authorization_url.contains(
        "scope=user-library-read%20user-follow-read%20playlist-read-private%20playlist-read-collaborative"
    ));

    let (issued_state, verifier) = {
        let states = state.oauth_states.read().await;
        let (issued_state, record) = states.records.iter().next().expect("issued state");
        (
            issued_state.clone(),
            record.code_verifier.clone().expect("PKCE verifier"),
        )
    };
    use base64::Engine as _;
    use sha2::Digest as _;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(super::Sha256::digest(verifier.as_bytes()));
    assert!(authorization_url.contains(&format!("code_challenge={challenge}")));
    assert!(!authorize.body.contains(&verifier));

    let invalid = super::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?code=abc&state=bogus",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid callback response");
    assert_eq!(invalid.status, "400 Bad Request");
    assert!(state
        .oauth_states
        .read()
        .await
        .records
        .contains_key(&issued_state));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_oauth_error_callback_returns_html_without_consuming_state() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
    );
    super::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    let issued_state = state
        .oauth_states
        .read()
        .await
        .records
        .keys()
        .next()
        .cloned()
        .expect("issued state");

    let missing = super::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?error=access_denied",
        None,
        "",
        &state,
    )
    .await
    .expect("missing state response");
    assert_eq!(missing.status, "200 OK");
    assert_eq!(missing.content_type, "text/html; charset=utf-8");
    assert!(missing.body.contains("Spotify authorization failed."));

    let invalid = super::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?error=access_denied&state=bogus",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid state response");
    assert_eq!(invalid.status, "200 OK");

    let error_path =
        format!("/api/integrations/spotify/callback?error=access_denied&state={issued_state}");
    let denied = super::route_http_request("GET", &error_path, None, "", &state)
        .await
        .expect("valid error callback");
    assert_eq!(denied.status, "200 OK");
    assert!(denied.body.contains("Spotify authorization failed."));

    let replay = super::route_http_request("GET", &error_path, None, "", &state)
        .await
        .expect("replayed error callback");
    assert_eq!(replay.status, "200 OK");
    assert!(state
        .oauth_states
        .read()
        .await
        .records
        .contains_key(&issued_state));
}
