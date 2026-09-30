use super::*;

async fn spawn_tls_webhook_fixture(
    certificate_host: &str,
) -> (SocketAddr, tokio::task::JoinHandle<bool>) {
    use rcgen::generate_simple_self_signed;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::{
        rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig},
        TlsAcceptor,
    };

    let certified = generate_simple_self_signed(vec![certificate_host.to_owned()]).unwrap();
    let certificate = certified.cert.der().clone();
    let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key.into())
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let Ok(mut tls) = acceptor.accept(tcp).await else {
            return false;
        };
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = tls.read(&mut buffer).await.unwrap();
            if read == 0 {
                return false;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                break;
            }
        }
        tls.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        true
    });
    (address, server)
}

#[tokio::test]
async fn frozen_compat_webhook_sends_custom_headers_and_retries() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for status in ["500 Internal Server Error", "204 No Content"] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            let header_end = loop {
                let read = stream.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap();
            while request.len() < header_end + length {
                let read = stream.read(&mut buffer).await.unwrap();
                request.extend_from_slice(&buffer[..read]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("POST /hook HTTP/1.1"), "{request}");
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: fixture-secret"),
                "{request}"
            );
            assert!(
                request.ends_with(r#"{"type":"PrivateMessageReceived"}"#),
                "{request}"
            );
            stream
                .write_all(
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let resolved = ResolvedWebhookTarget {
        host: "fixture.invalid".to_owned(),
        addrs: vec![address],
    };
    WebhookDispatcher::send_frozen_compat_webhook_resolved(
        "http://fixture.invalid/hook",
        &[("Authorization".to_owned(), "fixture-secret".to_owned())],
        r#"{"type":"PrivateMessageReceived"}"#,
        Duration::from_millis(500),
        2,
        false,
        &resolved,
    )
    .await
    .unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn frozen_compat_certificate_override_accepts_only_matching_self_issued_certificates() {
    let payload = r#"{"type":"PrivateMessageReceived"}"#;

    let (address, server) = spawn_tls_webhook_fixture("fixture.invalid").await;
    let resolved = ResolvedWebhookTarget {
        host: "fixture.invalid".to_owned(),
        addrs: vec![address],
    };
    WebhookDispatcher::send_frozen_compat_webhook_resolved(
        "https://fixture.invalid/hook",
        &[],
        payload,
        Duration::from_secs(2),
        1,
        true,
        &resolved,
    )
    .await
    .unwrap();
    assert!(server.await.unwrap());

    let (address, server) = spawn_tls_webhook_fixture("fixture.invalid").await;
    let resolved = ResolvedWebhookTarget {
        host: "fixture.invalid".to_owned(),
        addrs: vec![address],
    };
    assert!(WebhookDispatcher::send_frozen_compat_webhook_resolved(
        "https://fixture.invalid/hook",
        &[],
        payload,
        Duration::from_secs(2),
        1,
        false,
        &resolved,
    )
    .await
    .is_err());
    assert!(!server.await.unwrap());

    let (address, server) = spawn_tls_webhook_fixture("fixture.invalid").await;
    let resolved = ResolvedWebhookTarget {
        host: "wrong.invalid".to_owned(),
        addrs: vec![address],
    };
    assert!(WebhookDispatcher::send_frozen_compat_webhook_resolved(
        "https://wrong.invalid/hook",
        &[],
        payload,
        Duration::from_secs(2),
        1,
        true,
        &resolved,
    )
    .await
    .is_err());
    assert!(!server.await.unwrap());
}

#[test]
fn test_webhook_event_display() {
    assert_eq!(WebhookEvent::SearchCreated.to_string(), "search.created");
    assert_eq!(
        WebhookEvent::TransferStarted.to_string(),
        "transfer.started"
    );
    assert_eq!(WebhookEvent::MessageSent.to_string(), "message.sent");
}

#[test]
fn test_webhook_creation() {
    let secret = Webhook::generate_secret().expect("test randomness");
    validate_webhook_secret(&secret).expect("generated secret is strong enough");
    let webhook = Webhook::new(
        "http://example.com/hook".to_string(),
        vec![WebhookEvent::SearchCreated, WebhookEvent::TransferStarted],
        secret,
    );

    assert!(webhook.id.starts_with("hook_"));
    assert_ne!(webhook.id, "hook_0");
    assert_eq!(webhook.url, "http://example.com/hook");
    assert!(webhook.active);
    assert_eq!(webhook.max_retries, 3);
}

#[test]
fn webhook_secret_generation_fails_closed_when_randomness_is_unavailable() {
    assert!(Webhook::generate_secret_with(|_| false).is_none());

    let secret = Webhook::generate_secret_with(|bytes| {
        bytes.fill(0xcd);
        true
    })
    .expect("deterministic randomness fixture");
    assert_eq!(secret, format!("secret_{}", "cd".repeat(32)));
}

#[test]
fn test_webhook_secret_validation() {
    assert!(validate_webhook_secret("short").is_err());
    assert!(validate_webhook_secret("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
    assert!(validate_webhook_secret("abcdefghij0123456789ABCDEFGHIJ!!").is_ok());
    assert!(validate_webhook_secret("abcdefghij0123456789\nABCDEFGHIJ!!").is_err());
    assert!(validate_webhook_secret(&"abcdefgh".repeat(MAX_WEBHOOK_SECRET_BYTES / 8)).is_ok());
    assert!(validate_webhook_secret(&format!(
        "{}x",
        "abcdefgh".repeat(MAX_WEBHOOK_SECRET_BYTES / 8)
    ))
    .is_err());
}

#[test]
fn test_webhook_event_handling() {
    let secret = Webhook::generate_secret().expect("test randomness");
    let webhook = Webhook::new(
        "http://example.com/hook".to_string(),
        vec![WebhookEvent::SearchCreated],
        secret,
    );

    assert!(webhook.handles_event(WebhookEvent::SearchCreated));
    assert!(!webhook.handles_event(WebhookEvent::TransferStarted));
}

#[test]
fn test_webhook_payload_creation() {
    let payload = WebhookPayload::new(
        WebhookEvent::SearchCreated,
        "corr-123".to_string(),
        serde_json::json!({"query": "test"}),
    );

    assert!(payload.id.starts_with("evt_"));
    assert_ne!(payload.id, "evt_0");
    assert_eq!(payload.event, "search.created");
    assert_eq!(payload.correlation_id, "corr-123");
}

#[test]
fn test_webhook_signature_creation_and_verification() {
    let secret = "test-secret";
    let payload = b"test payload";

    let sig = WebhookSignature::create(payload, secret).unwrap();
    assert!(sig.verify(payload, secret).unwrap());
    assert!(!sig.verify(b"different", secret).unwrap());
}

#[test]
fn test_webhook_signature_authenticates_timestamp() {
    let secret = "test-secret";
    let payload = b"test payload";
    let mut signature = WebhookSignature::create(payload, secret).unwrap();

    signature.timestamp += 1;
    assert!(!signature.verify(payload, secret).unwrap());
}

#[test]
fn test_webhook_signature_header_format() {
    let secret = "test-secret";
    let payload = b"test payload";

    let sig = WebhookSignature::create(payload, secret).unwrap();
    let header = sig.as_header();

    assert!(header.contains("t="));
    assert_eq!(header.split(", ").count(), 2);
}

#[test]
fn test_webhook_manager() {
    let mut manager = WebhookManager::new();
    let secret = Webhook::generate_secret().expect("test randomness");

    let webhook = Webhook::new(
        "http://example.com/hook".to_string(),
        vec![WebhookEvent::SearchCreated],
        secret,
    );

    let id = webhook.id.clone();
    manager.register(webhook).expect("register webhook");

    assert!(manager.get(&id).is_some());
    assert_eq!(manager.list().len(), 1);

    let results = manager.get_for_event(WebhookEvent::SearchCreated);
    assert_eq!(results.len(), 1);

    manager.unregister(&id);
    assert!(manager.get(&id).is_none());
}

#[test]
fn webhook_manager_rejects_invalid_runtime_and_persisted_definitions() {
    let valid = Webhook::new(
        "https://example.com/hook".to_owned(),
        vec![WebhookEvent::SearchCreated],
        Webhook::generate_secret().expect("test randomness"),
    );
    let mut invalid = valid.clone();
    invalid.secret = "x".repeat(MAX_WEBHOOK_SECRET_BYTES + 1);

    let mut manager = WebhookManager::new();
    assert!(manager.register(invalid.clone()).is_err());
    assert!(manager.get_all().is_empty());

    let mut persisted = vec![invalid; MAX_WEBHOOKS];
    let valid_id = valid.id.clone();
    persisted.push(valid);
    let manager = WebhookManager::from_webhooks(persisted);
    assert_eq!(manager.get_all().len(), 1);
    assert!(manager.get(&valid_id).is_some());
}

#[test]
fn webhook_capacity_counts_unique_ids_and_allows_rotation() {
    let base = Webhook::new(
        "https://example.com/hook".to_owned(),
        vec![WebhookEvent::SearchCreated],
        Webhook::generate_secret().expect("test randomness"),
    );
    let mut manager = WebhookManager::new();
    for index in 0..MAX_WEBHOOKS {
        let mut webhook = base.clone();
        webhook.id = format!("hook-{index}");
        manager.register(webhook).expect("fill webhook capacity");
    }

    let mut rotated = base.clone();
    rotated.id = "hook-0".to_owned();
    rotated.url = "https://example.com/rotated".to_owned();
    manager
        .register(rotated)
        .expect("rotate an existing webhook at capacity");
    assert_eq!(
        manager.get("hook-0").unwrap().url,
        "https://example.com/rotated"
    );

    let mut extra = base.clone();
    extra.id = "hook-extra".to_owned();
    assert!(manager.register(extra).is_err());

    let mut persisted = Vec::new();
    for _ in 0..MAX_WEBHOOKS {
        let mut duplicate = base.clone();
        duplicate.id = "hook-0".to_owned();
        persisted.push(duplicate);
    }
    for index in 1..MAX_WEBHOOKS {
        let mut unique = base.clone();
        unique.id = format!("hook-{index}");
        persisted.push(unique);
    }
    let restored = WebhookManager::from_webhooks(persisted);
    assert_eq!(restored.get_all().len(), MAX_WEBHOOKS);
}

#[tokio::test]
async fn dispatch_does_not_spawn_when_delivery_pool_is_full() {
    dispatch_rejection_records_failed_delivery(true).await;
}

#[tokio::test]
async fn dispatch_rejects_closed_task_admission_and_records_failure() {
    dispatch_rejection_records_failed_delivery(false).await;
}

async fn dispatch_rejection_records_failed_delivery(pool_full: bool) {
    let manager = Arc::new(RwLock::new(WebhookManager::new()));
    let webhook = Webhook::new(
        "https://example.com/hook".to_owned(),
        vec![WebhookEvent::SearchCreated],
        Webhook::generate_secret().expect("test randomness"),
    );
    let webhook_id = webhook.id.clone();
    manager
        .write()
        .await
        .register(webhook.clone())
        .expect("register webhook");
    let database = DatabaseManager::in_memory().await.expect("in-memory db");
    database
        .insert_webhook(&crate::persistence::WebhookRecord {
            id: webhook_id.clone(),
            url: webhook.url.clone(),
            events: WebhookEvent::SearchCreated.to_string(),
            secret: webhook.secret.clone(),
            active: true,
            created_at: 1,
            last_triggered: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
        })
        .await
        .expect("persist webhook");
    database
        .insert_webhook_log(&crate::persistence::WebhookLogRecord {
            id: "log_pool_full".to_owned(),
            webhook_id: webhook_id.clone(),
            event: WebhookEvent::SearchCreated.to_string(),
            correlation_id: "correlation".to_owned(),
            status: "queued".to_owned(),
            request_body: "{}".to_owned(),
            response_status: None,
            response_body: None,
            error_message: None,
            attempt: 1,
            timestamp: 1,
        })
        .await
        .expect("persist queued log");
    let deliveries = Arc::new(Semaphore::new(usize::from(!pool_full)));

    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    if !pool_full {
        tasks.shutdown().await;
    }
    WebhookDispatcher::dispatch(
        &tasks,
        WebhookDispatchContext {
            manager: Arc::clone(&manager),
            deliveries: Arc::clone(&deliveries),
            persistence_turn: Arc::new(tokio::sync::Mutex::new(())),
            database: Some(database.clone()),
        },
        vec![webhook],
        "correlation".to_owned(),
        WebhookEvent::SearchCreated,
        serde_json::json!({"query": "bounded"}),
    )
    .await;

    assert_eq!(Arc::strong_count(&deliveries), 1);
    assert_eq!(deliveries.available_permits(), usize::from(!pool_full));
    let logs = database
        .get_webhook_logs(&webhook_id, 10, 0)
        .await
        .expect("read delivery log");
    assert_eq!(logs[0].status, "failed");
    assert_eq!(
        logs[0].error_message.as_deref(),
        Some(if pool_full {
            "webhook delivery pool is full"
        } else {
            "daemon task admission is closed"
        })
    );
    let manager = manager.read().await;
    let webhook = manager
        .get(&webhook_id)
        .expect("webhook remains registered");
    assert!(webhook.last_triggered.is_some());
    assert_eq!(webhook.retry_count, 0);
    let persisted = database
        .get_webhook(&webhook_id)
        .await
        .expect("read webhook statistics")
        .expect("webhook remains persisted");
    assert!(persisted.last_triggered.is_some());
    assert_eq!(persisted.retry_count, 0);
}

#[tokio::test]
async fn delivery_stats_wait_for_persistence_turn_before_mutating() {
    let manager = Arc::new(RwLock::new(WebhookManager::new()));
    let webhook = Webhook::new(
        "https://example.com/hook".to_owned(),
        vec![WebhookEvent::SearchCreated],
        Webhook::generate_secret().expect("test randomness"),
    );
    let webhook_id = webhook.id.clone();
    manager
        .write()
        .await
        .register(webhook.clone())
        .expect("register webhook");
    let database = DatabaseManager::in_memory().await.expect("in-memory db");
    database
        .insert_webhook(&crate::persistence::WebhookRecord {
            id: webhook_id.clone(),
            url: webhook.url.clone(),
            events: WebhookEvent::SearchCreated.to_string(),
            secret: webhook.secret.clone(),
            active: true,
            created_at: webhook.created_at,
            last_triggered: None,
            retry_count: 0,
            max_retries: i32::try_from(webhook.max_retries).unwrap_or(i32::MAX),
            timeout_seconds: i32::try_from(webhook.timeout_seconds).unwrap_or(i32::MAX),
        })
        .await
        .expect("persist webhook");

    let persistence_turn = Arc::new(tokio::sync::Mutex::new(()));
    let held_turn = persistence_turn.lock().await;
    let task_manager = Arc::clone(&manager);
    let task_turn = Arc::clone(&persistence_turn);
    let task_database = database.clone();
    let mut update = tokio::spawn(async move {
        WebhookDispatcher::record_delivery_stats(
            &task_manager,
            Some(&task_database),
            webhook,
            2,
            task_turn,
        )
        .await;
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let manager_read = manager.read().await;
    assert!(manager_read
        .get(&webhook_id)
        .expect("webhook remains registered")
        .last_triggered
        .is_none());
    let persisted = database
        .get_webhook(&webhook_id)
        .await
        .expect("read queued webhook")
        .expect("webhook remains persisted");
    assert!(persisted.last_triggered.is_none());
    drop(manager_read);

    drop(held_turn);
    update.await.expect("delivery stats persistence task");
    let manager_read = manager.read().await;
    let webhook = manager_read
        .get(&webhook_id)
        .expect("webhook remains registered");
    assert!(webhook.last_triggered.is_some());
    assert_eq!(webhook.retry_count, 2);
    drop(manager_read);
    let persisted = database
        .get_webhook(&webhook_id)
        .await
        .expect("read persisted webhook statistics")
        .expect("webhook remains persisted");
    assert!(persisted.last_triggered.is_some());
    assert_eq!(persisted.retry_count, 2);

    let previous = manager
        .read()
        .await
        .get(&webhook_id)
        .expect("webhook remains registered")
        .clone();
    database.close_for_test().await;
    WebhookDispatcher::record_delivery_stats(
        &manager,
        Some(&database),
        previous.clone(),
        3,
        Arc::new(tokio::sync::Mutex::new(())),
    )
    .await;
    let manager_read = manager.read().await;
    let current = manager_read
        .get(&webhook_id)
        .expect("failed write keeps webhook registered");
    assert_eq!(current.last_triggered, previous.last_triggered);
    assert_eq!(current.retry_count, previous.retry_count);
}

#[test]
fn test_constant_time_compare() {
    let a = b"test";
    let b_same = b"test";
    let b_diff = b"different";

    assert!(constant_time_compare(a, b_same));
    assert!(!constant_time_compare(a, b_diff));
    assert!(!constant_time_compare(a, b"te")); // Different length
}

#[test]
fn test_blocked_webhook_special_use_ip_ranges() {
    for address in ["100.64.0.1", "192.0.0.8", "192.88.99.1", "198.18.0.1"] {
        assert!(is_blocked_webhook_ip(address.parse().unwrap()));
    }
    assert!(is_blocked_webhook_ip("::ffff:127.0.0.1".parse().unwrap()));
    assert!(is_blocked_webhook_ip(
        "::ffff:192.168.1.10".parse().unwrap()
    ));
    assert!(is_blocked_webhook_ip("2002:c0a8:0101::1".parse().unwrap()));
    assert!(is_blocked_webhook_ip(
        "2001:0000:4136:e378::1".parse().unwrap()
    ));
    assert!(is_blocked_webhook_ip("2001:db8::1".parse().unwrap()));
    assert!(is_blocked_webhook_ip("ff02::1".parse().unwrap()));
    for address in [
        "64:ff9b::7f00:1",
        "64:ff9b:1::1",
        "100::1",
        "2001:2::1",
        "2001:10::1",
        "2001:20::1",
    ] {
        assert!(is_blocked_webhook_ip(address.parse().unwrap()));
    }
}

#[test]
fn test_webhook_outbound_policy_covers_operator_cidrs() {
    let policy = WebhookOutboundPolicy {
        allow_cidrs: vec![IpCidr::parse("10.42.0.0/16").unwrap()],
        deny_cidrs: vec![IpCidr::parse("93.184.216.0/24").unwrap()],
    };

    assert!(!policy.blocks("10.42.1.5".parse().unwrap()));
    assert!(policy.blocks("10.43.1.5".parse().unwrap()));
    assert!(policy.blocks("93.184.216.34".parse().unwrap()));
    assert!(!policy.blocks("93.184.217.34".parse().unwrap()));
    assert!(policy.blocks("2001:db8::42".parse().unwrap()));
    assert!(policy.blocks("ff02::1".parse().unwrap()));
}

#[test]
fn test_webhook_cidr_parser_matches_ipv4_and_ipv6_prefixes() {
    let v4 = IpCidr::parse("198.51.100.0/24").unwrap();
    assert!(v4.contains("198.51.100.23".parse().unwrap()));
    assert!(!v4.contains("198.51.101.23".parse().unwrap()));

    let v6 = IpCidr::parse("2001:db8:abcd::/48").unwrap();
    assert!(v6.contains("2001:db8:abcd::1".parse().unwrap()));
    assert!(!v6.contains("2001:db8:abce::1".parse().unwrap()));

    assert!(IpCidr::parse("10.0.0.0/33").is_err());
    assert!(IpCidr::parse("2001:db8::/129").is_err());
}

#[test]
fn test_webhook_registration_url_validation() {
    assert!(validate_webhook_url_for_registration("https://example.com/hook").is_ok());
    assert!(validate_webhook_url_for_registration(&format!(
        "https://example.com/{}",
        "x".repeat(MAX_WEBHOOK_URL_BYTES)
    ))
    .is_err());
    assert!(
        validate_webhook_url_for_registration("https://operator:secret@example.com/hook").is_err()
    );
    assert!(validate_webhook_url_for_registration("ftp://example.com/hook").is_err());
    assert!(validate_webhook_url_for_registration("http://localhost/hook").is_err());
    assert!(validate_webhook_url_for_registration("http://127.0.0.1/hook").is_err());
    assert!(validate_webhook_url_for_registration("http://10.0.0.5/hook").is_err());
    assert!(validate_webhook_url_for_registration("http://169.254.169.254/hook").is_err());
}

#[tokio::test]
async fn webhook_dns_resolution_is_bounded() {
    let error = resolve_webhook_addrs(
        std::future::pending::<std::io::Result<Vec<SocketAddr>>>(),
        Duration::ZERO,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "webhook DNS resolution timed out");
}

#[test]
fn test_webhook_retry_backoff_saturates_before_overflow() {
    for (attempt, seconds) in [(0, 30), (1, 60), (4, 480), (5, 480), (u32::MAX, 480)] {
        assert_eq!(
            WebhookDispatcher::calculate_backoff(attempt),
            std::time::Duration::from_secs(seconds)
        );
    }
}

#[test]
fn frozen_webhook_attempts_are_bounded() {
    assert_eq!(bounded_webhook_attempts(0), 1);
    assert_eq!(bounded_webhook_attempts(2), 2);
    assert_eq!(bounded_webhook_attempts(u32::MAX), WEBHOOK_MAX_RETRIES + 1);
}

#[tokio::test]
async fn send_webhook_with_retries_makes_a_single_attempt_when_max_retries_is_zero() {
    // Matches max_retries=0's real semantics (attempts = 0 + 1 = 1):
    // a permanently-blocked target (SSRF-filtered loopback address)
    // must fail immediately, with no retry-backoff sleep at all --
    // the real multi-attempt retry loop this fix adds must never
    // turn a single delivery attempt into a multi-second wait.
    let start = std::time::Instant::now();
    let result = WebhookDispatcher::send_webhook_with_retries(
        "http://127.0.0.1:1/unreachable",
        "secret",
        "{}",
        1,
        0,
    )
    .await;
    assert!(result.is_err());
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "a single attempt must not incur any retry-backoff delay"
    );
}

#[test]
fn test_sanitized_webhook_url_omits_secret_path_and_query() {
    assert_eq!(
        sanitized_webhook_url_for_log(
            "https://example.com/services/secret-path?token=secret-query"
        ),
        "https://example.com"
    );
    assert_eq!(
        sanitized_webhook_delivery_error(
            "request failed for https://example.com/hook?token=secret-query"
        ),
        "webhook delivery request failed"
    );
    assert_eq!(
        sanitized_webhook_delivery_error(
            "webhook delivery failed with status 503 Service Unavailable"
        ),
        "webhook delivery failed with status 503 Service Unavailable"
    );
}

#[test]
fn test_stale_webhook_signature_header_rejected() {
    let old = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        - 600;
    let err = WebhookSignature::from_header(&format!("t={old}, abc")).unwrap_err();
    assert!(err.to_string().contains("stale"), "{err}");

    let err = WebhookSignature::from_header(&format!("t={}, abc", i64::MIN)).unwrap_err();
    assert!(err.to_string().contains("stale"), "{err}");
}

#[tokio::test]
async fn managed_delivery_shutdown_closes_stalled_http_socket_and_returns_permit() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let deliveries = Arc::new(Semaphore::new(1));
    let permit = Arc::clone(&deliveries).acquire_owned().await.unwrap();
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    assert!(tasks.try_spawn(async move {
        let _permit = permit;
        let resolved = ResolvedWebhookTarget {
            host: "fixture.invalid".to_owned(),
            addrs: vec![address],
        };
        let _ = WebhookDispatcher::send_frozen_compat_webhook_resolved(
            "http://fixture.invalid/hook",
            &[],
            "{}",
            Duration::from_secs(30),
            1,
            false,
            &resolved,
        )
        .await;
    }));
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut bytes = [0; 1024];
    let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(read > 0, "delivery must reach its stalled HTTP server");
    assert_eq!(deliveries.available_permits(), 0);
    tasks.shutdown().await;
    assert_eq!(deliveries.available_permits(), 1);
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match stream.read(&mut bytes).await {
                Ok(0) => break,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
                    ) =>
                {
                    break
                }
                Err(error) => panic!("unexpected socket read failure: {error}"),
            }
        }
    })
    .await
    .expect("cancelled webhook must close its HTTP socket");
}
