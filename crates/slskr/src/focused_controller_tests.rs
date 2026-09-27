use std::{
    collections::BTreeMap,
    fs,
    sync::{Arc, RwLock as StdRwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use tokio::sync::{mpsc, RwLock};

use crate::config::{ConfigEnv, FileConfig};
use slskr_client::protocol::peer::FileEntry;

#[tokio::test]
async fn http_listener_connection_tasks_share_capacity_and_join_with_managed_shutdown() {
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    let connections = Arc::new(super::Semaphore::new(1));
    let registry = super::ManagedTaskRegistry::default();
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = DropMarker(Arc::clone(&dropped));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();

    assert!(registry.spawn_bounded_http(&connections, async move {
        let _marker = marker;
        let _ = started_tx.send(());
        std::future::pending::<()>().await;
    },));
    started_rx.await.expect("HTTPS task should start");

    assert!(
        !registry.spawn_bounded_http(&connections, async {}),
        "the Unix listener must share the HTTPS connection limit"
    );
    assert_eq!(connections.available_permits(), 0);

    registry.shutdown().await;
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(connections.available_permits(), 1);

    assert!(
        !registry.spawn_bounded_http(&connections, async {}),
        "connection handlers must not be accepted after managed shutdown"
    );
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

fn test_state_with_env(
    extra_env: MapEnv,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    test_state_with_env_and_db(extra_env, None)
}

fn test_state_with_db(
    extra_env: MapEnv,
    db: super::persistence::DatabaseManager,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    test_state_with_env_and_db(extra_env, Some(db))
}

fn test_state_with_env_and_db(
    extra_env: MapEnv,
    db: Option<super::persistence::DatabaseManager>,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-focused-route-test-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&state_dir).expect("create focused test state directory");
    let mut env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTH_DISABLED", "true")
        .with("SLSKR_API_RATE_LIMIT_ANONYMOUS", "1000")
        .with("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
        .with("SLSK_USERNAME", "tester")
        .with("SLSK_PASSWORD", "secret")
        .with("SLSKD_MUSICBRAINZ_BASE_URL", "http://127.0.0.1:9")
        .with("SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS", "0.05")
        .with("SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS", "1");
    env.values.extend(extra_env.values);
    let controller_cli_environment = env.values.clone();
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env)
        .expect("focused test config");
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
        controller_version: StdRwLock::new(super::ControllerVersionState::initial()),
        controller_cli_environment,
        log_level: RwLock::new(super::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(config.credentials()),
        controller_web_auth_username: StdRwLock::new(config.controller_web_auth_username.clone()),
        controller_web_auth_password: StdRwLock::new(config.controller_web_auth_password.clone()),
        controller_web_jwt_key_current: StdRwLock::new(config.controller_web_jwt_key.clone()),
        session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
        server_address: StdRwLock::new(config.server_address.clone()),
        connected_server_address: StdRwLock::new(None),
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
        downloads_dir: StdRwLock::new(config.downloads_dir.clone()),
        incomplete_dir: StdRwLock::new(config.incomplete_dir.clone()),
        download_completed_path_template: StdRwLock::new(
            config.download_completed_path_template.clone(),
        ),
        remote_file_management: StdRwLock::new(config.remote_file_management),
        remote_configuration: StdRwLock::new(config.remote_configuration),
        controller_no_config_watch: StdRwLock::new(config.controller_no_config_watch),
        controller_case_sensitive_regex: StdRwLock::new(config.controller_case_sensitive_regex),
        user_info_description: StdRwLock::new(config.user_info_description.clone()),
        user_info_picture: StdRwLock::new(config.user_info_picture.clone()),
        controller_options_validation_error: StdRwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: StdRwLock::new(config.advertised_port),
        obfuscated_advertised_port: StdRwLock::new(config.obfuscated_advertised_port),
        searches: RwLock::new(super::SearchStore::new()),
        users: RwLock::new(super::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(super::MeshState::new()),
        capability_signing_key: super::new_capability_signing_key().expect("capability key"),
        content_discovery: RwLock::new(super::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(super::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(super::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(super::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(super::MessageStore::new()),
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
            .expect("focused search filters"),
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
        rooms: RwLock::new(super::RoomStore::new()),
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
        event_tx,
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
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        db: db.clone(),
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
            None,
            distributed_persistence_receiver,
            distributed_persistence_status_sender,
        ));
    } else {
        drop(distributed_persistence_receiver);
        drop(distributed_persistence_status_sender);
    }
    (state, receiver)
}

fn write_ledger(file_name: &str, ledger: &[serde_json::Value]) {
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create focused evidence directory");
    fs::write(
        evidence_dir.join(file_name),
        serde_json::to_string_pretty(ledger).expect("serialize focused evidence"),
    )
    .expect("write focused evidence");
}

fn write_file_lifecycle_ledger(file_name: &str, ledger: &[serde_json::Value]) {
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create focused file evidence directory");
    fs::write(
        evidence_dir.join(file_name),
        serde_json::to_string_pretty(ledger).expect("serialize focused file evidence"),
    )
    .expect("write focused file evidence");
}

#[tokio::test]
async fn controller_api_differential_controller_file_transfer_room_residuals() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
    let downloads = state.config.downloads_dir.clone();
    let incomplete = state.config.incomplete_dir.clone();
    fs::create_dir_all(downloads.join("Artist/Album")).expect("downloads fixture");
    fs::write(downloads.join("Artist/Album/Track.flac"), b"track").expect("download fixture");
    fs::create_dir_all(incomplete.join("Partial")).expect("incomplete fixture");
    fs::write(incomplete.join("Partial/Track.part"), b"partial").expect("partial fixture");

    for (path, route) in [
        (
            "/api/v0/files/downloads/directories?recursive=true",
            "/api/v0/files/downloads/directories",
        ),
        (
            "/api/v0/files/incomplete/directories?recursive=true",
            "/api/v0/files/incomplete/directories",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("root storage list");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let pass = response.status == "200 OK"
            && json["directories"].is_array()
            && json["files"].is_array();
        record!("GET", route, "nominal-status-headers-body", pass);
        record!("GET", route, "populated-dynamic-state", pass);
    }

    for (path, route, expected_name) in [
        (
            "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
            "Track.flac",
        ),
        (
            "/api/v0/files/incomplete/directories/UGFydGlhbA==",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
            "Track.part",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("nested storage list");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let pass = response.status == "200 OK"
            && json["files"]
                .as_array()
                .is_some_and(|files| files.iter().any(|file| file["name"] == expected_name));
        record!("GET", route, "nominal-status-headers-body", pass);
        record!("GET", route, "populated-dynamic-state", pass);
    }

    let missing_nested = super::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TWlzc2luZw==",
        None,
        "",
        &state,
    )
    .await
    .expect("missing nested storage list");
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state",
        missing_nested.status == "404 Not Found"
    );

    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "Remote/Delete.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RemoveDownload",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Partial/Remove.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RemovePartial",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let root = if storage == "downloads" {
            &downloads
        } else {
            &incomplete
        };
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("delete fixture parent"))
            .expect("delete fixture directory");
        if resource == "directories" {
            fs::create_dir_all(&path).expect("delete fixture directory path");
            fs::write(path.join("delete-me.bin"), b"delete me")
                .expect("delete fixture directory file");
        } else {
            fs::write(&path, b"delete me").expect("delete fixture file");
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(relative);
        let request_path = format!("/api/v0/files/{storage}/{resource}/{encoded}");
        let deleted = super::route_http_request("DELETE", &request_path, None, "", &state)
            .await
            .expect("delete storage path");
        let nominal = deleted.status == "204 No Content" && !path.exists();
        record!("DELETE", route, "nominal-status-headers-body", nominal);
        record!(
            "DELETE",
            route,
            "mutation-side-effects-and-readback",
            nominal
        );
        let repeated = super::route_http_request("DELETE", &request_path, None, "", &state)
            .await
            .expect("repeat delete storage path");
        let expected_repeated_status = if resource == "files" {
            "204 No Content"
        } else {
            "404 Not Found"
        };
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            repeated.status == expected_repeated_status
        );
    }

    for (storage, resource, encoded, route) in [
        (
            "downloads",
            "files",
            base64::engine::general_purpose::STANDARD.encode("Missing.mp3"),
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "incomplete",
            "files",
            base64::engine::general_purpose::STANDARD.encode("Missing.part"),
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/files/{storage}/{resource}/{encoded}"),
            None,
            "",
            &state,
        )
        .await
        .expect("missing storage delete");
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            response.status == "204 No Content"
        );
    }
    let traversal = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            base64::engine::general_purpose::STANDARD.encode("../secret")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("traversal storage delete");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body",
        traversal.status == "400 Bad Request"
    );

    let reset_dir = state.config.state_dir.display().to_string();
    let (reset_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_STATE_DIR", &reset_dir),
    );
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "Remote/Reset.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "ResetDownload",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Reset.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "ResetIncomplete",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                base64::engine::general_purpose::STANDARD.encode(relative)
            ),
            None,
            "",
            &reset_state,
        )
        .await
        .expect("reset storage delete");
        record!(
            "DELETE",
            route,
            "restart-persistence-or-reset",
            response.status
                == if resource == "files" {
                    "204 No Content"
                } else {
                    "404 Not Found"
                }
        );
    }

    let downloads_conflict = std::env::temp_dir().join(format!(
        "slskr-focused-download-conflict-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let incomplete_conflict = std::env::temp_dir().join(format!(
        "slskr-focused-incomplete-conflict-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&downloads_conflict, b"not a directory").expect("downloads conflict");
    fs::write(&incomplete_conflict, b"not a directory").expect("incomplete conflict");
    let (failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    *failure_state
        .downloads_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict.clone();
    *failure_state
        .incomplete_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict.clone();
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "RuntimeFailure.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "RuntimeFailure.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                base64::engine::general_purpose::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .expect("storage runtime failure");
        record!(
            "DELETE",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }
    #[cfg(unix)]
    {
        for storage in ["downloads", "incomplete"] {
            let target_dir = std::env::temp_dir().join(format!(
                "slskr-focused-list-target-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            ));
            let link = std::env::temp_dir().join(format!(
                "slskr-focused-list-link-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            ));
            fs::create_dir_all(&target_dir).expect("create storage list target");
            std::os::unix::fs::symlink(&target_dir, &link).expect("create storage list symlink");
            let state_root = if storage == "downloads" {
                &failure_state.downloads_dir
            } else {
                &failure_state.incomplete_dir
            };
            *state_root
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = link.clone();
            let response = super::route_http_request(
                "GET",
                &format!("/api/v0/files/{storage}/directories"),
                None,
                "",
                &failure_state,
            )
            .await
            .expect("storage root runtime failure");
            record!(
                "GET",
                if storage == "downloads" {
                    "/api/v0/files/downloads/directories"
                } else {
                    "/api/v0/files/incomplete/directories"
                },
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
            let _ = fs::remove_file(link);
            let _ = fs::remove_dir_all(target_dir);
        }
        *failure_state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict.clone();
        *failure_state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict.clone();
    }
    for (storage, route) in [
        (
            "downloads",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "GET",
            &format!(
                "/api/v0/files/{storage}/directories/{}",
                base64::engine::general_purpose::STANDARD.encode("RuntimeFailureDirectory")
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .expect("nested storage runtime failure");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }
    let _ = fs::remove_file(downloads_conflict);
    let _ = fs::remove_file(incomplete_conflict);

    let enqueue = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        r#"{"files":[{"filename":"Remote/Queued.flac","size":99}]}"#,
        &state,
    )
    .await
    .expect("enqueue focused transfer");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap_or_default();
    let transfer_id = enqueue_json["transfers"][0]["id"]
        .as_str()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| enqueue_json["transfers"][0]["id"].as_u64())
        .expect("focused transfer id");
    let enqueue_pass = enqueue.status == "200 OK"
        && enqueue_json["queued"] == 1
        && enqueue_json["transfers"][0]["username"] == "peer";
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body",
        enqueue_pass
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "mutation-side-effects-and-readback",
        enqueue_pass
    );

    let list = super::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
        .await
        .expect("focused transfer list");
    let list_json = serde_json::from_str::<serde_json::Value>(&list.body).unwrap_or_default();
    let list_pass = list.status == "200 OK"
        && list_json
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["username"] == "peer"));
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "nominal-status-headers-body",
        list_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "populated-dynamic-state",
        list_pass
    );

    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("focused transfer detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    let detail_pass = detail.status == "200 OK"
        && (detail_json["id"] == serde_json::json!(transfer_id)
            || detail_json["id"] == serde_json::json!(transfer_id.to_string()));
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body",
        detail_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "populated-dynamic-state",
        detail_pass
    );
    let missing_detail = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/other/999999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing focused transfer detail");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );
    let cancelled = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel focused transfer");
    let cancel_pass = cancelled.status == "204 No Content";
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body",
        cancel_pass
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "mutation-side-effects-and-readback",
        cancel_pass
    );

    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let batch_body = format!(
        r#"{{"id":"{batch_id}","username":"peer","files":[{{"filename":"Music/A.flac","size":42}}]}}"#
    );
    let batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("focused transfer batch");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or_default();
    let batch_pass = batch.status == "201 Created" && batch_json["batch"]["id"] == batch_id;
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "nominal-status-headers-body",
        batch_pass
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "mutation-side-effects-and-readback",
        batch_pass
    );
    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("duplicate focused transfer batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "missing-empty-or-conflict-state",
        duplicate.status == "409 Conflict"
    );

    state.session.write().await.state = "connected";
    let available = super::route_http_request("GET", "/api/v0/rooms/available", None, "", &state)
        .await
        .expect("focused available rooms");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/available",
        "nominal-status-headers-body",
        available.status == "200 OK" && available_json.is_array()
    );
    for (path, route) in [
        (
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "/api/v0/rooms/joined/missing/users",
            "/api/v0/rooms/joined/{roomName}/users",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("missing focused room subresource");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    assert!(
        mismatches.is_empty(),
        "{} focused slskd controller mismatches: {}",
        mismatches.len(),
        mismatches.join("; ")
    );
    write_ledger(
        "controller_focused_file_transfer_room_residuals.json",
        &ledger,
    );
}

#[tokio::test]
async fn room_join_fast_path_does_not_return_stale_duplicate_projection() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state.session.write().await.state = "connected";

    let mut rooms = state.rooms.write().await;
    let healthy = rooms.join("music".to_owned()).expect("healthy room");
    rooms.records[0].joined = false;
    rooms.records[0].last_error = Some("stale join failure".to_owned());
    rooms.records.push(healthy);
    drop(rooms);

    let response = super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("room join response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"lastError\":null"));
    assert!(!response.body.contains("stale join failure"));
}

#[test]
fn folder_contents_response_parser_accepts_controller_wire_shape() {
    let entries =
        crate::config::parse_share_entries("open-commons-fixtures/commons-click-track.ogg=168370")
            .expect("fixture share entry");
    let payload = super::build_folder_contents_payload(
        &entries,
        7,
        "open-commons-fixtures",
        Default::default(),
    )
    .expect("folder response payload");
    let parsed = super::folder_entries_from_peer_message(
        super::PeerMessage::FolderContentsResponse(payload),
        "open-commons-fixtures",
        Default::default(),
    )
    .expect("parse folder response");
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].filename,
        "open-commons-fixtures/commons-click-track.ogg"
    );
    assert_eq!(parsed[0].size, 168370);
}

#[test]
fn folder_request_replaces_previous_browse_entries() {
    let mut browse = super::BrowseStore::new();
    browse.request("friend".to_owned()).expect("browse record");
    browse.add_entries(
        "friend".to_owned(),
        vec![super::BrowseEntry {
            path_encoding: Default::default(),
            filename: "open-commons-fixtures/commons-click-track.ogg".to_owned(),
            size: 168370,
            extension: "ogg".to_owned(),
        }],
        true,
    );
    let requested = browse
        .request_folder("friend".to_owned(), "open-commons-fixtures".to_owned())
        .expect("folder record");
    assert!(requested.entries.is_empty());
}

#[tokio::test]
async fn batch_operations_reuse_router_statuses_timeouts_and_nested_guard() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let missing = super::route_http_request("GET", "/api/not-a-route", None, "", &state)
        .await
        .expect("direct missing-route response");
    assert_eq!(missing.status, "404 Not Found");
    let body = serde_json::json!({
        "operations": [
            {"id": "health", "method": "GET", "path": "/api/health"},
            {"id": "version", "method": "GET", "path": "/api/version"},
            {"id": "nested", "method": "POST", "path": "/api/v0/batch"},
            {"id": "missing", "method": "GET", "path": "/api/not-a-route"}
        ]
    })
    .to_string();
    let response = super::route_http_request("POST", "/api/batch", None, &body, &state)
        .await
        .expect("batch response");
    assert_eq!(response.status, "202 Accepted");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).expect("batch JSON");
    assert_eq!(json["accepted"], true);
    assert_eq!(json["results"].as_array().map(Vec::len), Some(4));
    assert_eq!(json["executed"], 2);
    assert_eq!(json["failed"], 2);
    assert!(json["total_time_ms"].is_u64());
    assert_eq!(json["results"][0]["status"], 200);
    assert_eq!(json["results"][2]["status"], 400);
    assert_eq!(json["results"][3]["status"], 404);

    let stop_on_error = serde_json::json!({
        "operations": [
            {"id": "ok", "method": "GET", "path": "/api/health"},
            {"id": "missing", "method": "GET", "path": "/api/not-a-route"},
            {"id": "skipped", "method": "GET", "path": "/api/version"}
        ],
        "config": {"continueOnError": false}
    })
    .to_string();
    let response = super::route_http_request("POST", "/api/batch", None, &stop_on_error, &state)
        .await
        .expect("batch stop-on-error response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).expect("batch JSON");
    assert_eq!(json["results"].as_array().map(Vec::len), Some(2));
    assert_eq!(json["executed"], 1);
    assert_eq!(json["failed"], 1);
}

#[tokio::test]
async fn unversioned_empty_search_put_persists_cancellation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "id": "search-stop-fixture",
        "query": "stop-me"
    })
    .to_string();
    let created = super::route_http_request("POST", "/api/searches", None, &create, &state)
        .await
        .expect("create unversioned search");
    assert_eq!(created.status, "200 OK");

    let stopped =
        super::route_http_request("PUT", "/api/searches/search-stop-fixture", None, "", &state)
            .await
            .expect("stop unversioned search");
    assert_eq!(stopped.status, "200 OK");

    let record =
        super::route_http_request("GET", "/api/searches/search-stop-fixture", None, "", &state)
            .await
            .expect("read stopped search");
    let json = serde_json::from_str::<serde_json::Value>(&record.body).expect("search JSON");
    assert_eq!(json["status"], "cancelled");
    assert_eq!(json["state"], "Cancelled");
    assert!(json["endedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[tokio::test]
async fn wishlist_completion_releases_guard_when_item_is_missing() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let item_id = {
        let mut wishlist = state.wishlist.write().await;
        wishlist
            .add_item("Artist".to_owned(), "Track".to_owned(), "Audio".to_owned())
            .expect("wishlist item")
            .id
    };
    let token = {
        let mut searches = state.searches.write().await;
        searches
            .create_scheduled_wishlist_for_item("one two".to_owned(), Some(item_id.clone()), 300)
            .expect("wishlist search")
            .record
            .token
    };
    assert!(state.wishlist.write().await.remove_item(&item_id).is_some());

    let response = tokio::time::timeout(
        Duration::from_secs(1),
        super::route_http_request(
            "POST",
            &format!("/api/searches/{token}/complete"),
            None,
            "",
            &state,
        ),
    )
    .await
    .expect("wishlist completion must not wait on its own write guard")
    .expect("wishlist completion response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"status\":\"completed\""));
}

#[tokio::test]
async fn file_lifecycle_differential_controller_file_service_existing_missing_overwrite() {
    let target = "slskd";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let managed_file = state
        .config
        .downloads_dir
        .join("FileService")
        .join("managed.bin");
    fs::create_dir_all(managed_file.parent().expect("managed file parent"))
        .expect("create managed file parent");
    fs::write(&managed_file, b"managed-file").expect("write managed file");
    let encoded = base64::engine::general_purpose::STANDARD.encode("FileService/managed.bin");
    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{encoded}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete managed file");
    let missing = super::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{encoded}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing managed file");
    let pass = deleted.status == "204 No Content"
        && missing.status == "204 No Content"
        && !managed_file.exists();
    assert!(
        pass,
        "slskd FileService delete contract: first={}, second={}, exists={}",
        deleted.status,
        missing.status,
        managed_file.exists()
    );
    write_file_lifecycle_ledger(
        "controller_focused_file_service_existing_missing_overwrite.json",
        &[serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "existing-missing-and-overwrite",
            "pass": pass,
        })],
    );
}

#[test]
fn security_authorization_matrix_matches_declared_policy_for_every_frozen_route() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        access: String,
        scheme: String,
        scopes: Vec<String>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Outcome {
        Allowed,
        Unauthorized,
        Forbidden,
    }

    #[derive(Clone, Copy)]
    struct Profile {
        name: &'static str,
        header: Option<&'static str>,
        credential: Option<(u8, &'static str, bool)>,
    }

    const PROFILES: [Profile; 10] = [
        Profile {
            name: "anonymous",
            header: None,
            credential: None,
        },
        Profile {
            name: "basic-readonly",
            header: Some("ApiKey read-token"),
            credential: Some((0, "api_key", false)),
        },
        Profile {
            name: "basic-readwrite",
            header: Some("ApiKey write-token"),
            credential: Some((1, "api_key", false)),
        },
        Profile {
            name: "basic-administrator",
            header: Some("ApiKey admin-token"),
            credential: Some((2, "api_key", false)),
        },
        Profile {
            name: "bearer-readonly",
            header: Some("Bearer read-token"),
            credential: Some((0, "jwt", false)),
        },
        Profile {
            name: "bearer-readwrite",
            header: Some("Bearer write-token"),
            credential: Some((1, "jwt", false)),
        },
        Profile {
            name: "bearer-administrator",
            header: Some("Bearer admin-token"),
            credential: Some((2, "jwt", false)),
        },
        Profile {
            name: "invalid-or-expired-credential",
            header: Some("Bearer not-a-real-differential-token"),
            credential: None,
        },
        Profile {
            name: "missing-required-scope",
            header: Some("ApiKey nowplaying-token"),
            credential: Some((1, "api_key", true)),
        },
        Profile {
            name: "wrong-authentication-scheme",
            header: None,
            credential: None,
        },
    ];

    fn required_access_rank(access: &str) -> Option<u8> {
        match access {
            "anonymous" | "delegated" => None,
            "administrator" => Some(2),
            "read_write" => Some(1),
            _ => Some(0),
        }
    }

    fn expected_outcome(rule: &AuthPolicyRow, profile: Profile) -> Outcome {
        let Some(required) = required_access_rank(&rule.access) else {
            return Outcome::Allowed;
        };
        let Some((credential_rank, credential_scheme, nowplaying_only)) = profile.credential else {
            return Outcome::Unauthorized;
        };
        if credential_rank < required {
            return Outcome::Forbidden;
        }
        if rule.scheme != "any" && credential_scheme != rule.scheme {
            return Outcome::Forbidden;
        }
        let requires_nowplaying = rule.scopes.iter().any(|scope| scope == "nowplaying");
        if nowplaying_only && !requires_nowplaying {
            return Outcome::Forbidden;
        }
        Outcome::Allowed
    }

    fn placeholder_path(route: &str) -> String {
        let mut segments: Vec<String> = route
            .trim_matches('/')
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    "differential-fixture-value".to_owned()
                } else {
                    segment.to_owned()
                }
            })
            .collect();
        if route.contains("{*") {
            segments.push("differential-fixture-tail".to_owned());
        }
        format!("/{}", segments.join("/"))
    }

    let headers = super::RequestSecurityHeaders::default();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    for (target, source) in [
        (
            "slskd",
            include_str!("../data/legacy-controller-auth-policy.json"),
        ),
        (
            "slskdn",
            include_str!("../data/native-controller-auth-policy.json"),
        ),
    ] {
        let rules: Vec<AuthPolicyRow> =
            serde_json::from_str(source).expect("checked profile auth policy registry");
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-focused-security-auth-{target}-{}",
            uuid::Uuid::new_v4()
        ));
        let config = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", state_dir.to_str().expect("state path"))
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token")
                .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
                .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
                .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token"),
        )
        .expect("hermetic auth-policy differential config");

        for rule in &rules {
            let path = placeholder_path(&rule.route);
            for profile in PROFILES {
                let (header, expected) = if profile.name == "wrong-authentication-scheme" {
                    if rule.scheme == "jwt" {
                        (Some("ApiKey admin-token"), Outcome::Forbidden)
                    } else if rule.scheme == "api_key" {
                        (Some("Bearer admin-token"), Outcome::Forbidden)
                    } else {
                        let any_scheme_profile = Profile {
                            header: Some("Bearer admin-token"),
                            credential: Some((2, "jwt", false)),
                            ..profile
                        };
                        (
                            any_scheme_profile.header,
                            expected_outcome(rule, any_scheme_profile),
                        )
                    }
                } else {
                    (profile.header, expected_outcome(rule, profile))
                };
                let actual = match super::routing::check_route_auth(
                    &config,
                    &rule.method,
                    &path,
                    header,
                    &headers,
                ) {
                    Ok(()) => Outcome::Allowed,
                    Err("unauthorized") => Outcome::Unauthorized,
                    Err("forbidden") => Outcome::Forbidden,
                    Err(other) => panic!(
                        "unexpected auth-gate outcome {other:?} for {target} {} {}",
                        rule.method, rule.route
                    ),
                };
                let pass = actual == expected;
                if !pass {
                    mismatches.push(format!(
                        "{target} {} {} [{}]: expected {expected:?}, got {actual:?}",
                        rule.method, rule.route, profile.name
                    ));
                }
                ledger.push(serde_json::json!({
                    "target": target,
                    "method": rule.method,
                    "route": rule.route,
                    "case": profile.name,
                    "pass": pass,
                    "expected": format!("{expected:?}"),
                    "actual": format!("{actual:?}"),
                }));
            }
        }
        let _ = fs::remove_dir_all(&state_dir);
    }

    let evidence_dir = std::env::temp_dir().join("slskr-parity-evidence");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("security-authorization.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security-authorization ledger"),
    )
    .expect("write security-authorization ledger");

    assert!(
        mismatches.is_empty(),
        "{} security-authorization mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[tokio::test]
async fn application_projection_exposes_selected_runtime_profile() {
    for (reference_fixture, expected_profile) in [("slskd", "legacy"), ("slskdn", "native")] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", reference_fixture),
        );
        let response = super::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("application projection");
        let body = serde_json::from_str::<serde_json::Value>(&response.body)
            .expect("application projection JSON");

        assert_eq!(response.status, "200 OK");
        assert_eq!(body["runtimeProfile"], expected_profile);
    }
}

#[tokio::test]
async fn watched_obfuscation_changes_mark_reconnect_once_while_connected() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state.session.write().await.state = "connected";
    let yaml = "soulseek:\n  obfuscation:\n    enabled: false\n    mode: prefer\n    listen_port: 50302\n    advertise_regular_port: false\n    prefer_outbound: false\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_reconnect_pending);
    state.runtime.write().await.set_reconnect_pending(false);

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(!state.runtime.read().await.application_reconnect_pending);
}

#[tokio::test]
async fn watched_share_reload_cancels_and_rejects_stale_index_publication() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create watched-share database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let stale_snapshot = state.shares.read().await.clone();
    let stale_generation = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);
    let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&cancellation));

    let new_root = state.config.state_dir.join("watched-share");
    fs::create_dir_all(&new_root).expect("create reloaded share root");
    fs::write(new_root.join("new.flac"), b"new share").expect("write reloaded share fixture");
    let yaml = format!(
        "shares:\n  directories:\n    - '[Reloaded]{}'\n",
        new_root.display()
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &yaml)
        .expect("write watched share configuration");
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.remove("SLSKR_SHARE_FIXTURE");

    super::apply_watched_controller_configuration(&state, Some(&yaml), &cli_environment).await;

    assert!(cancellation.load(std::sync::atomic::Ordering::Acquire));
    let generation_after_directory_reload = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);

    let hidden_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(Arc::clone(&hidden_cancellation));
    let mut hidden_environment = cli_environment.clone();
    hidden_environment.insert("SLSKR_SHARE_INCLUDE_HIDDEN".to_owned(), "true".to_owned());
    super::apply_watched_controller_configuration(&state, Some(&yaml), &hidden_environment).await;
    assert!(hidden_cancellation.load(std::sync::atomic::Ordering::Acquire));
    assert!(state.share_settings.read().await.include_hidden);
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > generation_after_directory_reload
    );
    let generation_after_hidden_reload = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);

    let regex_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&regex_cancellation));
    let mut regex_environment = hidden_environment.clone();
    regex_environment.insert("SLSKD_CASE_SENSITIVE_REGEX".to_owned(), "true".to_owned());
    super::apply_watched_controller_configuration(&state, Some(&yaml), &regex_environment).await;
    assert!(regex_cancellation.load(std::sync::atomic::Ordering::Acquire));
    assert!(*state
        .controller_case_sensitive_regex
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner));
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > generation_after_hidden_reload
    );
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > stale_generation
    );
    assert!(state.share_lifecycle.read().await.scan_pending);

    assert_eq!(
        super::commit_share_index_snapshot_checked(&state, &stale_snapshot, stale_generation)
            .await
            .expect_err("old scan generation must be rejected"),
        super::SHARE_SCAN_CANCELLED_ERROR
    );
    assert!(db
        .list_share_files(100, 0)
        .await
        .expect("read persisted shares")
        .is_empty());
    let shares = state.shares.read().await;
    assert_eq!(shares.roots[0].label, "Reloaded");
    drop(shares);
    let lifecycle = state.share_lifecycle.read().await;
    assert!(!lifecycle.scanning);
    assert!(lifecycle.cancelled);
    assert!(lifecycle.scan_pending);
    drop(lifecycle);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn controller_debug_view_projects_frozen_default_authentication_values() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let overlay = state.options_overlay.read().await;
    let debug = super::controller_options_debug_view(&state, &overlay);

    assert!(state.config.controller_metrics_password.is_empty());
    assert!(state
        .controller_web_auth_password
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty());
    assert_eq!(
        debug
            .matches("password=slskd (DefaultValueConfigurationProvider)")
            .count(),
        2
    );
}

#[tokio::test]
async fn versioned_search_accepts_web_acquisition_profiles() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for profile in [
        "lossless-exact",
        "fast-good-enough",
        "album-complete",
        "rare-hunt",
        "conservative-network",
        "mesh-preferred",
        "metadata-strict",
    ] {
        let body = serde_json::json!({
            "searchText": "profile-validation",
            "acquisitionProfile": profile,
        })
        .to_string();
        let response = super::route_http_request("POST", "/api/v0/searches", None, &body, &state)
            .await
            .expect("versioned search response");
        assert_eq!(
            response.status, "409 Conflict",
            "{profile}: {}",
            response.body
        );
    }

    let rejected = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"searchText":"profile-validation","acquisitionProfile":"made-up-profile"}"#,
        &state,
    )
    .await
    .expect("rejected versioned search response");
    assert_eq!(rejected.status, "400 Bad Request");
    assert!(rejected.body.contains("known acquisition profile"));
}

#[tokio::test]
async fn versioned_swarm_rejects_oversized_transfer_limits_before_discovery() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let oversized = super::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":17592186044417}"#,
        &state,
    )
    .await
    .expect("oversized versioned swarm response");
    assert_eq!(oversized.status, "400 Bad Request");
    assert!(oversized.body.contains("size exceeds"));

    let oversized_chunks = super::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":42,"chunkSize":16777216}"#,
        &state,
    )
    .await
    .expect("oversized versioned swarm chunk response");
    assert_eq!(oversized_chunks.status, "400 Bad Request");
    assert!(oversized_chunks.body.contains("chunkSize must be between"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn versioned_swarm_rejects_oversized_source_batches_before_deserialization() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    let oversized_sources = (0..super::multisource::MAX_SOURCES + 1)
        .map(|index| {
            serde_json::json!({
                "username": format!("peer-{index}"),
                "url": "https://source.example/file"
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        &serde_json::json!({
            "filename": "Track.flac",
            "size": 42,
            "expectedHash": "a".repeat(64),
            "sources": oversized_sources
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized versioned swarm source response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("source count exceeds the 16 source limit"));
    assert!(state.multisource.read().await.list().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn versioned_download_range_sources_use_verified_executor() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    let response = super::route_http_request(
        "POST",
        "/api/v0/multisource/download",
        None,
        &serde_json::json!({
            "filename": "Track.flac",
            "fileSize": 42,
            "expectedHash": "00".repeat(32),
            "sources": [
                {"username": "alice", "url": "http://127.0.0.1:9/file"},
                {"username": "bob", "url": "http://127.0.0.1:9/file"}
            ]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("versioned download response");
    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"success\":false"));
    assert!(!response
        .body
        .contains("Multi-source download is unavailable"));
    let multisource_store = state.multisource.read().await;
    let jobs = multisource_store.list();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "failed");

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn versioned_swarm_requires_expected_hash_before_queueing() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let response = super::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":42}"#,
        &state,
    )
    .await
    .expect("missing expected hash response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("expectedHash is required for verified swarm execution"));
    assert!(state.multisource.read().await.list().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn merge_routes_reject_oversized_arrays_before_store_deserialization() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let oversized_hash_body = serde_json::json!({
        "entries": vec![serde_json::json!({}); super::content_discovery::MAX_MESH_MERGE_ENTRIES + 1]
    })
    .to_string();
    let hash_response = super::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &oversized_hash_body,
        &state,
    )
    .await
    .expect("oversized hash merge response");
    assert_eq!(hash_response.status, "400 Bad Request");
    assert!(hash_response.body.contains("at most 2000 entries"));

    let oversized_records_body = serde_json::json!({
        "records": vec![serde_json::json!({}); super::content_discovery::MAX_SHADOW_MERGE_RECORDS + 1]
    })
    .to_string();
    let records_response = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &oversized_records_body,
        &state,
    )
    .await
    .expect("oversized shadow merge response");
    assert_eq!(records_response.status, "400 Bad Request");
    assert!(records_response.body.contains("at most 256 records"));

    let oversized_indexes_body = serde_json::json!({
        "records": [{"recordingId":"bounded-route-test","peerIds":[]}],
        "realmIndexes": vec![serde_json::json!({}); super::realm_subject_index::MAX_INDEXES + 1]
    })
    .to_string();
    let indexes_response = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &oversized_indexes_body,
        &state,
    )
    .await
    .expect("oversized realm-index merge response");
    assert_eq!(indexes_response.status, "400 Bad Request");
    assert!(indexes_response.body.contains("at most 1024 indexes"));

    let oversized_nested_index = serde_json::json!({
        "id": "nested-limit",
        "realmId": super::realm_subject_index::DEFAULT_REALM_ID,
        "subjectNamespace": "music",
        "revision": 1,
        "entries": vec![serde_json::json!({
            "subjectId": "nested-limit",
            "workRef": {
                "domain": "music",
                "title": "Nested Limit"
            }
        }); super::realm_subject_index::MAX_ENTRIES_PER_INDEX + 1]
    });
    let nested_response = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"nested-limit","peerIds":["peer-a"]}],
            "realmIndexes": [oversized_nested_index]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized nested realm-index response");
    assert_eq!(nested_response.status, "400 Bad Request");
    assert!(
        nested_response.body.contains("at most 10000 entries"),
        "{}",
        nested_response.body
    );
    assert!(state
        .realm_subject_indexes
        .read()
        .await
        .indexes_for_realm(super::realm_subject_index::DEFAULT_REALM_ID)
        .is_empty());
    assert!(state
        .content_discovery
        .read()
        .await
        .shadow_records()
        .iter()
        .all(|record| record.recording_id != "nested-limit"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn shadow_merge_rolls_back_records_when_realm_index_persistence_fails() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    *state.content_discovery.write().await =
        super::content_discovery::ContentDiscoveryStore::load(&state.config.state_dir)
            .expect("load file-backed content-discovery store");
    *state.realm_subject_indexes.write().await =
        super::realm_subject_index::Store::load_with_identity(
            &state.config.state_dir,
            super::realm_subject_index::DEFAULT_REALM_ID,
            [super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("load file-backed realm-index store");
    fs::create_dir(state.config.state_dir.join("realm-subject-indexes.json"))
        .expect("make realm-index persistence fail");

    let mut index = serde_json::json!({
        "id": "index-persistence-failure",
        "realmId": super::realm_subject_index::DEFAULT_REALM_ID,
        "subjectNamespace": "music",
        "revision": 1,
        "publishedAt": "2026-08-01T00:00:00Z",
        "entries": [{
            "subjectId": "subject-persistence-failure",
            "workRef": {
                "domain": "music",
                "title": "Persistence Failure",
                "creator": "Artist",
            },
            "externalIds": {
                "musicbrainz:recording": "recording-persistence-failure",
                "discogs": "discogs-persistence-failure",
            },
            "aliases": ["persistence-failure-alias"],
        }],
        "signature": {
            "signer": super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
            "algorithm": "realm-governance-sha256",
            "payloadHash": "",
            "value": "signature",
        },
    });
    index["signature"]["payloadHash"] =
        serde_json::json!(super::realm_subject_index::compute_payload_hash(&index));

    let response = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-persistence-failure","peerIds":["peer-a"]}],
            "realmIndexes": [index],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("failed shadow/realm-index merge response");

    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("realm subject-index storage is unavailable"));
    assert!(state
        .content_discovery
        .read()
        .await
        .shadow_records()
        .is_empty());
    assert!(state
        .realm_subject_indexes
        .read()
        .await
        .indexes_for_realm(super::realm_subject_index::DEFAULT_REALM_ID)
        .is_empty());
    let persisted = super::content_discovery::ContentDiscoveryStore::load(&state.config.state_dir)
        .expect("reload rolled-back shadow records");
    assert!(persisted.shadow_records().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn hash_db_writers_wait_before_mutating_and_commit_in_memory_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create HashDb ordering database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "slskdn")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = super::hash_db_persistence_turn().await;
    let store_state = Arc::clone(&state);
    let mut store = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/hashdb/hash",
            None,
            r#"{"filename":"Ordered/Stored.flac","size":4096,"byteHash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","sampleRate":44100,"channels":2,"bitDepth":16}"#,
            &store_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut store)
        .await
        .is_err());

    let merge_state = Arc::clone(&state);
    let merge_body = serde_json::json!({
        "entries": [{
            "flacKey": "ordered-merge-key",
            "byteHash": "b".repeat(64),
            "size": 4097,
        }]
    })
    .to_string();
    let mut merge = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/hashdb/sync/merge",
            None,
            &merge_body,
            &merge_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut merge)
        .await
        .is_err());

    assert!(state
        .content_discovery
        .read()
        .await
        .hash_entries()
        .is_empty());
    let readable =
        super::route_http_request("GET", "/api/v0/hashdb/hash/by-size/4096", None, "", &state)
            .await
            .expect("HashDb reads stay responsive while mutations wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["count"],
        0
    );
    assert!(db
        .list_hash_db_entries()
        .await
        .expect("read empty HashDb before release")
        .is_empty());

    drop(persistence_turn);
    assert_eq!(
        store
            .await
            .expect("HashDb store completes")
            .expect("store HashDb entry")
            .status,
        "200 OK"
    );
    assert_eq!(
        merge
            .await
            .expect("HashDb merge completes")
            .expect("merge HashDb entry")
            .status,
        "200 OK"
    );

    let memory = state.content_discovery.read().await;
    assert_eq!(memory.hash_entries().len(), 2);
    let latest_seq = memory.latest_seq();
    drop(memory);
    let persisted = db
        .list_hash_db_entries()
        .await
        .expect("read final HashDb rows");
    assert_eq!(persisted.len(), 2);
    let persisted_latest_seq = db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read final HashDb sequence")
        .expect("persisted HashDb sequence")
        .value
        .expect("sequence value")
        .parse::<u64>()
        .expect("numeric sequence");
    assert_eq!(persisted_latest_seq, latest_seq);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn hash_db_history_backfill_waits_before_advancing_persisted_progress() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create HashDb history backfill database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "slskdn")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    {
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(super::SearchRecord {
                id: format!("history-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: Vec::new(),
                raw_response_count: 1,
                filtered_out_count: 0,
                ignored_result_count: 0,
                hidden_locked_count: 0,
                fallback_attempts: 0,
                ttl_seconds: super::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let persistence_turn = super::hash_db_persistence_turn().await;
    let route_state = Arc::clone(&state);
    let mut first_backfill = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/hashdb/backfill/from-history?batchSize=10",
            None,
            "",
            &route_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut first_backfill)
            .await
            .is_err()
    );
    let route_state = Arc::clone(&state);
    let mut second_backfill = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/hashdb/backfill/from-history?batchSize=10",
            None,
            "",
            &route_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut second_backfill)
            .await
            .is_err()
    );

    assert!(state
        .controller_features
        .read()
        .await
        .get("hashdb/backfill/progress")
        .is_none());
    assert!(db
        .get_hash_db_state("hashdb/backfill/progress")
        .await
        .expect("read queued backfill cursor")
        .is_none());
    let readable = super::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("HashDb reads stay responsive during queued backfill");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["count"],
        0
    );

    drop(persistence_turn);
    let first_response = first_backfill
        .await
        .expect("first history backfill completes")
        .expect("first history backfill request");
    assert_eq!(first_response.status, "200 OK");
    let first_json = serde_json::from_str::<serde_json::Value>(&first_response.body).unwrap();
    assert_eq!(first_json["searchesProcessed"], 10);
    assert_eq!(first_json["remainingSearches"], 1);

    let second_response = second_backfill
        .await
        .expect("second history backfill completes")
        .expect("second history backfill request");
    assert_eq!(second_response.status, "200 OK");
    let second_json = serde_json::from_str::<serde_json::Value>(&second_response.body).unwrap();
    assert_eq!(second_json["searchesProcessed"], 1);
    assert_eq!(second_json["remainingSearches"], 0);
    assert_eq!(second_json["complete"], true);
    let persisted = db
        .get_hash_db_state("hashdb/backfill/progress")
        .await
        .expect("read persisted backfill cursor")
        .expect("backfill cursor was stored");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            persisted.value.as_deref().expect("backfill cursor value")
        )
        .unwrap()["lastProcessedAt"],
        1
    );

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

async fn seed_old_message_for_database_cleanup(
    state: &Arc<super::AppState>,
    db: &super::persistence::DatabaseManager,
) -> super::MessageRecord {
    let record = {
        let mut messages = state.messages.write().await;
        let mut record = messages.add(
            "cleanup-peer".to_owned(),
            "inbound",
            "old message".to_owned(),
        );
        record.created_at = 1;
        record.created_at_ms = 1_000;
        record.updated_at = 1;
        *messages
            .records
            .iter_mut()
            .find(|stored| stored.id == record.id)
            .expect("message remains in cleanup projection") = record.clone();
        record
    };
    db.insert_message(&super::message_store::persisted_message_record(&record))
        .await
        .expect("persist old message before database cleanup");
    record
}

#[tokio::test]
async fn database_cleanup_deletes_terminal_transfers_and_persists_tombstones() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create database cleanup persistence database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Remote/Complete.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "completed", Some(10), None)
            .expect("mark cleanup transfer completed")
    };
    super::persist_transfer_record(&state, &transfer)
        .await
        .expect("persist terminal transfer before cleanup");

    let cleanup = super::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "ok");
    assert_eq!(cleanup["pruned_transfers"], 1);
    assert_eq!(cleanup["transferCleanup"]["healthy"], true);
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .all(|entry| entry.id != transfer.id));
    assert!(db
        .list_transfers(None, 10, 0)
        .await
        .expect("read transfers after cleanup")
        .is_empty());
    assert_eq!(
        db.max_transfer_id()
            .await
            .expect("read transfer id retained by tombstone"),
        transfer.id
    );

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn database_cleanup_restores_terminal_transfers_when_sqlite_delete_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create database cleanup rollback database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let old_message = seed_old_message_for_database_cleanup(&state, &db).await;
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "failed", Some(1), Some("offline".to_owned()))
            .expect("mark cleanup transfer failed")
    };
    super::persist_transfer_record(&state, &transfer)
        .await
        .expect("persist terminal transfer before failed cleanup");
    db.close_for_test().await;

    let cleanup = super::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "error");
    assert_eq!(cleanup["transferCleanup"]["healthy"], false);
    assert_eq!(
        cleanup["transferCleanup"]["error"],
        "transfer cleanup unavailable"
    );
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.id == transfer.id));
    assert_eq!(cleanup["pruned_messages"], 0);
    assert!(state
        .messages
        .read()
        .await
        .records
        .iter()
        .any(|record| record.id == old_message.id));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_mutation_rollback_preserves_newer_rows() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let original = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Original.flac".to_owned(),
            None,
            Some(10),
        )
    };
    let (previous, mutated, failed_creation_id) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        transfers
            .update_status(
                original.id,
                "cancelled",
                None,
                Some("failed cancellation".to_owned()),
            )
            .expect("mutate original transfer");
        let failed_creation = transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Failed-Creation.flac".to_owned(),
            None,
            Some(11),
        );
        let mutated = transfers.mutation_snapshot();
        (previous, mutated, failed_creation.id)
    };
    let concurrent = {
        let mut transfers = state.transfers.write().await;
        transfers
            .update_status(original.id, "in_progress", Some(4), None)
            .expect("apply newer transfer update");
        transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Concurrent.flac".to_owned(),
            None,
            Some(12),
        )
    };

    assert!(super::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    {
        let transfers = state.transfers.read().await;
        let original_after = transfers
            .entries
            .iter()
            .find(|entry| entry.id == original.id)
            .expect("newer original transfer remains");
        assert_eq!(original_after.status, "in_progress");
        assert_eq!(original_after.bytes_transferred, 4);
        assert!(transfers
            .entries
            .iter()
            .all(|entry| entry.id != failed_creation_id));
        assert!(transfers
            .entries
            .iter()
            .any(|entry| entry.id == concurrent.id));
        assert!(transfers.next_id > concurrent.id);
    }

    let (previous, mutated) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        transfers
            .update_status(
                concurrent.id,
                "cancelled",
                None,
                Some("failed cancellation".to_owned()),
            )
            .expect("mutate concurrent transfer");
        (previous, transfers.mutation_snapshot())
    };
    assert!(super::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    let restored = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.id == concurrent.id)
        .cloned()
        .expect("restored transfer remains");
    assert_eq!(restored.status, "queued");

    let deleted = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Failed-Deletion.flac".to_owned(),
            None,
            Some(13),
        )
    };
    let (previous, mutated) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        assert_eq!(transfers.remove_entries(&[deleted.id]).len(), 1);
        (previous, transfers.mutation_snapshot())
    };
    let concurrent_after_delete = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Concurrent-After-Deletion.flac".to_owned(),
            None,
            Some(14),
        )
    };
    assert!(super::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    {
        let transfers = state.transfers.read().await;
        assert!(transfers.entries.iter().any(|entry| entry == &deleted));
        assert!(transfers
            .entries
            .iter()
            .any(|entry| entry.id == concurrent_after_delete.id));
    }

    let stale_expected = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Changed-Before-Removal.flac".to_owned(),
            None,
            Some(15),
        );
        transfers
            .update_status(entry.id, "in_progress", Some(5), None)
            .expect("update staged transfer before rollback");
        entry
    };
    assert!(
        super::remove_transfer_entries_if_unchanged(&state, &[stale_expected.clone()])
            .await
            .is_empty()
    );
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.id == stale_expected.id && entry.status == "in_progress"));

    let removable = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Unchanged-Staged-Transfer.flac".to_owned(),
            None,
            Some(16),
        )
    };
    assert_eq!(
        super::remove_transfer_entries_if_unchanged(&state, std::slice::from_ref(&removable)).await,
        vec![removable.clone()]
    );
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .all(|entry| entry.id != removable.id));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn database_cleanup_orders_message_projection_with_sqlite_deletion() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create message cleanup database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let old_message = seed_old_message_for_database_cleanup(&state, &db).await;
    let persistence_turn = state.message_persistence_lock.lock().await;
    let cleanup_state = Arc::clone(&state);
    let mut cleanup_task =
        tokio::spawn(
            async move { super::database_cleanup_value(&cleanup_state, "{\"days\":0}").await },
        );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut cleanup_task)
            .await
            .is_err()
    );

    let readable = super::route_http_request("GET", "/api/messages", None, "", &state)
        .await
        .expect("message reads stay responsive while cleanup waits");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(readable_json["count"], 1);
    assert_eq!(readable_json["entries"][0]["id"], old_message.id);
    assert_eq!(
        db.list_messages(10, 0)
            .await
            .expect("message rows remain while cleanup waits")
            .len(),
        1
    );

    drop(persistence_turn);
    let cleanup = cleanup_task.await.expect("message cleanup task completes");
    assert_eq!(cleanup["status"], "ok");
    assert_eq!(cleanup["cleaned"], 1);
    assert_eq!(cleanup["pruned_messages"], 1);
    assert_eq!(
        state.messages.read().await.records.len(),
        0,
        "message projection is pruned with SQLite"
    );
    assert!(db
        .list_messages(10, 0)
        .await
        .expect("read messages after cleanup")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn browse_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_entries = (0..=super::MAX_BROWSE_WIRE_FILES_PER_RESPONSE)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let entries_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-entries",
            "entries": oversized_entries
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse entries response");
    assert_eq!(entries_response.status, "400 Bad Request");
    assert!(entries_response
        .body
        .contains("browse response exceeds wire entry limits"));
    assert!(state.browse.read().await.get("oversized-entries").is_none());

    let oversized_directory_files = super::MAX_BROWSE_WIRE_FILES_PER_RESPONSE / 2 + 1;
    let nested_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-nested-files",
            "directories": [
                {
                    "name": "one",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("one-{index}.flac")}))
                        .collect::<Vec<_>>()
                },
                {
                    "name": "two",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("two-{index}.flac")}))
                        .collect::<Vec<_>>()
                }
            ]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized nested browse entries response");
    assert_eq!(nested_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-nested-files")
        .is_none());

    let oversized_directories = (0..=super::MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE)
        .map(|index| serde_json::json!({"name": format!("folder-{index}")}))
        .collect::<Vec<_>>();
    let directories_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-directories",
            "directories": oversized_directories
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse directory response");
    assert_eq!(directories_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-directories")
        .is_none());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn search_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let search_response = super::route_http_request(
        "POST",
        "/api/searches",
        None,
        r#"{"query":"oversized-response"}"#,
        &state,
    )
    .await
    .expect("create search for oversized response test");
    assert_eq!(search_response.status, "200 OK", "{}", search_response.body);
    let token = state
        .searches
        .read()
        .await
        .records
        .first()
        .map(|record| record.token)
        .expect("created search token");

    let oversized_files = (0..=super::MAX_SEARCH_RESULTS_PER_SEARCH)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &serde_json::json!({"token": token, "files": oversized_files}).to_string(),
        &state,
    )
    .await
    .expect("oversized search response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("search response exceeds result limits"));

    let searches = state.searches.read().await;
    let record = searches
        .records
        .iter()
        .find(|record| record.token == token)
        .expect("search remains present");
    assert!(record.results.is_empty());
    assert_eq!(record.raw_response_count, 0);
    drop(searches);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_reorder_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (collection_id, item_id) = {
        let mut collections = state.collections.write().await;
        let collection = collections
            .create(
                "tester".to_owned(),
                "Reorder bounds".to_owned(),
                String::new(),
            )
            .expect("create collection");
        let item = collections
            .add_item(
                &collection.id,
                "content-1".to_owned(),
                "Artist".to_owned(),
                "Track".to_owned(),
                "Audio".to_owned(),
            )
            .expect("add collection item")
            .expect("collection item");
        (collection.id, item.id)
    };

    let oversized_ids = (0..=super::MAX_COLLECTION_ITEMS)
        .map(|index| serde_json::json!(format!("item-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder"),
        None,
        &serde_json::json!({"itemIds": oversized_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized collection reorder");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("collection reorder exceeds item limits"));

    let collections = state.collections.read().await;
    let record = collections
        .get(&collection_id)
        .expect("collection remains present");
    assert_eq!(record.items.len(), 1);
    assert_eq!(record.items[0].id, item_id);
    drop(collections);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn musicbrainz_rejects_oversized_json_batches_before_state_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_recording_ids = (0..=super::MAX_LIBRARY_ITEMS)
        .map(|index| serde_json::json!(format!("recording-{index}")))
        .collect::<Vec<_>>();
    let diff_response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/diffs",
        None,
        &serde_json::json!({"recordingIds": oversized_recording_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz diff");
    assert_eq!(diff_response.status, "400 Bad Request");
    assert!(diff_response
        .body
        .contains("recordingIds must contain at most"));

    let oversized_suggestions = (0..=super::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!({"artist": "Artist", "title": format!("Release {index}")}))
        .collect::<Vec<_>>();
    let wishlist_response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/wishlist",
        None,
        &serde_json::json!({"suggestions": oversized_suggestions}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz wishlist");
    assert_eq!(wishlist_response.status, "400 Bad Request");
    assert!(wishlist_response
        .body
        .contains("suggestions must contain at most"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn failed_library_runtime_transaction_rolls_back_unchanged_store_independently() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let previous_library = state.library.read().await.clone();
    let previous_runtime = state.runtime.read().await.clone();

    let (mutated_library, mutated_runtime, failed_library_item) = {
        let mut library = state.library.write().await;
        let mut runtime = state.runtime.write().await;
        let record = library
            .create(
                "Artist".to_owned(),
                "Failed import".to_owned(),
                "Audio".to_owned(),
            )
            .expect("failed import candidate should fit");
        runtime.record_lidarr_manual_import(1, false, "/music/failed".to_owned(), Vec::new());
        (library.clone(), runtime.clone(), record)
    };

    // Model a runtime-only write that succeeds after the cross-store SQL
    // transaction fails but before its error path reacquires the state locks.
    let concurrent_runtime = {
        let mut runtime = state.runtime.write().await;
        runtime.record_gc();
        runtime.clone()
    };
    super::route_dispatch::rollback_library_runtime_if_unchanged(
        &state,
        previous_library.clone(),
        mutated_library,
        previous_runtime.clone(),
        mutated_runtime,
        failed_library_item.clone(),
    )
    .await;
    assert_eq!(*state.library.read().await, previous_library);
    let mut expected_runtime = concurrent_runtime;
    expected_runtime.lidarr_manual_imports = previous_runtime.lidarr_manual_imports;
    assert_eq!(*state.runtime.read().await, expected_runtime);

    // A concurrent library-only write likewise must survive while the failed
    // transaction's otherwise unchanged runtime candidate is rolled back.
    let (mutated_library, mutated_runtime, failed_library_item) = {
        let mut library = state.library.write().await;
        let mut runtime = state.runtime.write().await;
        let record = library
            .create(
                "Artist".to_owned(),
                "Second failed import".to_owned(),
                "Audio".to_owned(),
            )
            .expect("second failed import candidate should fit");
        runtime.record_lidarr_manual_import(
            1,
            false,
            "/music/second-failed".to_owned(),
            Vec::new(),
        );
        (library.clone(), runtime.clone(), record)
    };
    let previous_runtime = state.runtime.read().await.clone();
    let concurrent_library = {
        let mut library = state.library.write().await;
        library
            .create(
                "Artist".to_owned(),
                "Concurrent library write".to_owned(),
                "Audio".to_owned(),
            )
            .expect("concurrent library candidate should fit");
        library.clone()
    };
    let mut expected_library = concurrent_library.clone();
    expected_library
        .records
        .retain(|record| record.id != failed_library_item.id);
    super::route_dispatch::rollback_library_runtime_if_unchanged(
        &state,
        previous_library.clone(),
        mutated_library,
        previous_runtime.clone(),
        mutated_runtime,
        failed_library_item,
    )
    .await;
    assert_eq!(*state.library.read().await, expected_library);
    assert_eq!(*state.runtime.read().await, previous_runtime);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn runtime_compatibility_mutation_waits_for_persistence_turn_without_blocking_reads() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let persistence_turn = state.runtime_persistence_lock.lock().await;
    let task_state = Arc::clone(&state);
    let mut mutation = tokio::spawn(async move {
        super::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
    });

    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut mutation)
            .await
            .is_err()
    );
    assert_eq!(state.runtime.read().await.gc_runs, 0);

    drop(persistence_turn);
    mutation
        .await
        .expect("runtime mutation task should join")
        .expect("runtime mutation should persist");
    assert_eq!(state.runtime.read().await.gc_runs, 1);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn runtime_compatibility_mutation_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-runtime-compat-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = super::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create runtime compatibility database");
    db.execute_raw_for_test(
        "CREATE TRIGGER fail_runtime_compat_insert BEFORE INSERT ON runtime_compat_state \
         BEGIN SELECT RAISE(ABORT, 'forced runtime compatibility failure'); END",
    )
    .await
    .expect("install persistence failure trigger");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let mutation = tokio::spawn(async move {
        super::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
    });
    let read_saw_mutation = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if let Ok(runtime) = state.runtime.try_read() {
                if runtime.gc_runs == 1 {
                    break true;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mutation_waited_for_sqlite = !mutation.is_finished();
    let unrelated_update_survived = tokio::time::timeout(Duration::from_millis(200), async {
        state.runtime.write().await.application_reconnect_pending = true;
    })
    .await
    .is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let mutation_result = tokio::time::timeout(Duration::from_secs(2), mutation)
        .await
        .expect("runtime mutation should finish after releasing SQLite")
        .expect("runtime mutation task should join");
    assert!(
        mutation_result.is_err(),
        "trigger should fail the persistence write"
    );
    assert!(
        read_saw_mutation,
        "runtime reads should proceed during SQLite I/O"
    );
    assert!(
        mutation_waited_for_sqlite,
        "the write should wait on SQLite"
    );
    assert!(
        unrelated_update_survived,
        "unrelated runtime updates should proceed during SQLite I/O"
    );
    let runtime = state.runtime.read().await;
    assert_eq!(
        runtime.gc_runs, 0,
        "failed persistent mutation should roll back"
    );
    assert!(runtime.application_reconnect_pending);
    drop(runtime);

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[tokio::test]
async fn runtime_compatibility_transient_latches_stay_process_local() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create runtime compatibility database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    super::mutate_runtime_compat_state_in_memory(&state, |runtime| {
        runtime.set_restart_requested(true);
        runtime.bridge_running = true;
    })
    .await;
    super::mutate_runtime_compat_state(&state, |runtime, _| runtime.record_gc())
        .await
        .expect("persist durable runtime counter");

    let live = state.runtime.read().await;
    assert!(live.application_restart_requested);
    assert!(live.bridge_running);
    drop(live);

    let persisted = db
        .get_runtime_compat_state()
        .await
        .expect("read runtime compatibility state")
        .expect("durable runtime state was persisted");
    assert_eq!(persisted.gc_runs, 1);
    assert!(!persisted.application_restart_requested);
    assert!(!persisted.bridge_running);

    let rehydrated = super::RuntimeCompatState::try_from_persisted(&persisted)
        .expect("rehydrate durable runtime state");
    assert!(!rehydrated.application_restart_requested);
    assert!(!rehydrated.bridge_running);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_rejects_oversized_file_batches_before_queue_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let files = (0..=super::MAX_TRANSFER_REQUEST_FILES)
        .map(|index| serde_json::json!({"filename": format!("Music/{index}.flac"), "size": 1}))
        .collect::<Vec<_>>();
    let body = serde_json::json!({"username": "peer", "files": files}).to_string();
    for path in [
        "/api/v0/transfers/downloads/batches",
        "/api/v0/transfers/downloads/peer",
        "/api/transfers",
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized transfer request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains("file limits"),
            "{path}: {}",
            response.body
        );
    }
    assert!(state.transfers.read().await.entries.is_empty());
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn string_array_routes_reject_oversized_wire_batches_before_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let recipients = (0..=super::MAX_PRIVATE_MESSAGE_RECIPIENTS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/conversations/batch",
        None,
        &serde_json::json!({"usernames": recipients, "body": "hello"}).to_string(),
        &state,
    )
    .await
    .expect("oversized conversation batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("conversation batch exceeds recipient limits"));
    assert!(state.messages.read().await.records.is_empty());

    let item_ids = (0..=super::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!(format!("00000000-0000-0000-0000-{index:012x}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "PUT",
        "/api/v0/wishlist/bulk-filter",
        None,
        &serde_json::json!({"itemIds": item_ids, "filter": "flac"}).to_string(),
        &state,
    )
    .await
    .expect("oversized wishlist filter batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(
        response
            .body
            .contains("wishlist bulk filter exceeds item limits"),
        "{}",
        response.body
    );
    assert!(state.wishlist.read().await.records.is_empty());

    let capabilities = (0..=super::MAX_CAPABILITY_NEGOTIATION_ITEMS)
        .map(|index| serde_json::json!(format!("capability-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/capabilities/negotiate",
        None,
        &serde_json::json!({"capabilities": capabilities}).to_string(),
        &state,
    )
    .await
    .expect("oversized capability negotiation");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("capabilities negotiation exceeds item limits"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn mediacore_rejects_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let descriptors = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    let content_ids = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("content:test:{index}")))
        .collect::<Vec<_>>();
    let keys = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("cache-key-{index}")))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/publish/batch",
            serde_json::json!({"descriptors": descriptors}).to_string(),
            "descriptors must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/publish/republish",
            serde_json::json!({"contentIds": content_ids.clone()}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/batch",
            serde_json::json!({"contentIds": content_ids}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/cache/clear",
            serde_json::json!({"keys": keys}).to_string(),
            "keys must contain at most 100 items",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn remaining_controller_array_routes_reject_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());

    let download_items = (0..=super::MAX_EXTENDED_DOWNLOAD_ITEMS)
        .map(|index| {
            serde_json::json!({
                "user": "peer",
                "remotePath": format!("Music/{index}.flac"),
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/downloads",
        None,
        &serde_json::json!({"items": download_items}).to_string(),
        &state,
    )
    .await
    .expect("oversized extended download request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("items must contain at most 1000 items"));
    assert!(state.transfers.read().await.entries.is_empty());

    let links = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "name": format!("link-{index}"),
                "target": format!("target-{index}"),
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:test:oversized",
        None,
        &serde_json::json!({"links": links}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore links request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("links must contain at most 100 items"));

    let entries = (0..=super::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/portability/analyze",
            serde_json::json!({"package": {"entries": entries.clone()}}).to_string(),
            "entries must contain at most 1000 items",
        ),
        (
            "/api/v0/mediacore/portability/import",
            serde_json::json!({"package": {"entries": entries}}).to_string(),
            "entries must contain at most 1000 items",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore portability request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let content_ids = (0..=super::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!(format!("content:test:export:{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/export",
        None,
        &serde_json::json!({"contentIds": content_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore export request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("contentIds must contain at most 1000 items"));

    let sources = (0..=super::multisource::MAX_SOURCES)
        .map(|index| serde_json::json!({"username": format!("peer-{index}")}))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/multisource/test",
        None,
        &serde_json::json!({"sources": sources}).to_string(),
        &state,
    )
    .await
    .expect("oversized multisource test request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("source count exceeds the 16 source limit"));

    let jurors = (0..=super::MAX_QUARANTINE_JURY_ITEMS)
        .map(|index| serde_json::json!(format!("juror-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        &serde_json::json!({
            "localReason": "test",
            "jurors": jurors,
            "evidence": [{"reference": "safe-reference", "summary": "test"}],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized quarantine request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("quarantine jury arrays must contain at most 100 items"));

    let usernames = (0..=super::MAX_RANKING_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        &serde_json::to_string(&usernames).expect("ranking usernames JSON"),
        &state,
    )
    .await
    .expect("oversized ranking history request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 usernames"));

    let candidates = (0..=super::MAX_RANKING_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "username": format!("peer-{index}"),
                "filename": "Music/test.flac",
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        &serde_json::to_string(&candidates).expect("ranking candidates JSON"),
        &state,
    )
    .await
    .expect("oversized ranking request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 source candidates"));

    let issue_ids = (0..=25)
        .map(|index| serde_json::json!(format!("issue-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/slskdn/library/remediate",
        None,
        &serde_json::json!({"issue_ids": issue_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized library remediation request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("issue_ids must contain 1 to 25 values"));

    let muted_release_group_ids = (0..=super::MAX_RADAR_MUTED_RELEASE_GROUPS)
        .map(|index| serde_json::json!(format!("release-group-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &serde_json::json!({
            "artistId": "artist:test",
            "mutedReleaseGroupIds": muted_release_group_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized release-radar subscription request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("mutedReleaseGroupIds must contain at most 256 items"));

    let target_peer_ids = (0..=super::MAX_ROUTING_TARGET_PEERS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {"messageId": "message:test", "channelId": "channel:test"},
            "targetPeerIds": target_peer_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore routing request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("targetPeerIds must contain at most 256 items"));

    let tags = (0..=super::MAX_POD_DISCOVERY_TAGS)
        .map(|index| serde_json::json!(format!("tag-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/register",
        None,
        &serde_json::json!({
            "podId": "pod:test",
            "visibility": "listed",
            "name": "test",
            "tags": tags,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore discovery request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("tags must contain at most 100 items"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_csv_import_rejects_oversized_row_batches() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_csv = (0..=super::MAX_CSV_IMPORT_ROWS)
        .map(|index| format!("artist-{index},title-{index}"))
        .collect::<Vec<_>>()
        .join("\n");

    let unversioned_response = super::route_http_request(
        "POST",
        "/api/wishlist/import/csv",
        None,
        &oversized_csv,
        &state,
    )
    .await
    .expect("oversized unversioned CSV import");
    assert_eq!(unversioned_response.status, "400 Bad Request");
    assert!(unversioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));

    let versioned_response = super::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        &serde_json::json!({"csvText": oversized_csv}).to_string(),
        &state,
    )
    .await
    .expect("oversized versioned CSV import");
    assert_eq!(versioned_response.status, "400 Bad Request");
    assert!(versioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    let preview_response = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &serde_json::json!({
            "sourceText": oversized_csv,
            "sourceKind": "csv",
            "fetchProviderUrls": false,
            "limit": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized source preview");
    assert_eq!(preview_response.status, "400 Bad Request");
    assert!(preview_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn library_browser_projects_share_tree_and_sha256_stream_ids() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let root = std::env::temp_dir().join(format!(
        "slskr-browser-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("Album")).expect("create browser fixture directory");
    let local_file = root.join("Album").join("Track.flac");
    fs::write(&local_file, b"browser stream fixture").expect("write browser fixture");
    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Local/Album/Track.flac".to_owned(),
            size: fs::metadata(&local_file)
                .expect("browser fixture metadata")
                .len(),
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
        shares
            .local_paths
            .insert("Local/Album/Track.flac".to_owned(), local_file.clone());
    }

    let root_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser root response");
    assert_eq!(root_response.status, "200 OK");
    let root_body =
        serde_json::from_str::<serde_json::Value>(&root_response.body).expect("browser root JSON");
    assert_eq!(root_body["files"].as_array().unwrap().len(), 0);
    assert!(root_body["directories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|directory| directory["path"] == "Local"));

    let folder_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio&path=Local",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser folder response");
    assert_eq!(folder_response.status, "200 OK");
    let folder_body = serde_json::from_str::<serde_json::Value>(&folder_response.body)
        .expect("browser folder JSON");
    let directories = folder_body["directories"].as_array().unwrap();
    assert_eq!(directories[0]["path"], "Local/Album");
    assert_eq!(directories[0]["fileCount"], 1);

    let album_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio&path=Local%2FAlbum",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser album response");
    let album_body = serde_json::from_str::<serde_json::Value>(&album_response.body)
        .expect("browser album JSON");
    let file = &album_body["files"][0];
    assert_eq!(file["fileName"], "Track.flac");
    let content_id = file["contentId"].as_str().unwrap();
    assert!(content_id.starts_with("sha256:"));
    assert!(super::open_primary_stream_file(&state, content_id, None)
        .await
        .expect("open sha256 stream")
        .is_some());

    let encoded_content_id = content_id.replace(':', "%3A");
    let stream_status = super::route_http_request(
        "GET",
        &format!("/api/v0/streams/{encoded_content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("sha256 stream status");
    assert_eq!(stream_status.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stream_status.body).unwrap()["status"],
        "available"
    );

    let ticket = super::route_http_request(
        "POST",
        &format!("/api/v0/streams/{encoded_content_id}/ticket"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("sha256 stream ticket");
    assert_eq!(ticket.status, "200 OK", "{}", ticket.body);

    let invalid_path = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?path=..",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid browser path response");
    assert_eq!(invalid_path.status, "400 Bad Request");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn profile_static_roots_serve_the_selected_spa_on_dashboard() {
    let root = std::env::temp_dir().join(format!(
        "slskr-profile-static-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("legacy")).expect("create legacy profile root");
    fs::create_dir_all(root.join("native")).expect("create native profile root");
    fs::write(root.join("legacy/index.html"), "legacy-ui").expect("write legacy index");
    fs::write(root.join("native/index.html"), "native-ui").expect("write native index");

    let (legacy_root, legacy_index, _) = super::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .expect("legacy dashboard SPA root");
    let (native_root, native_index, _) = super::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(super::ControllerProfile::Native),
    )
    .expect("native dashboard SPA root");

    assert!(legacy_root.ends_with("legacy"));
    assert!(legacy_index.ends_with("legacy/index.html"));
    assert!(native_root.ends_with("native"));
    assert!(native_index.ends_with("native/index.html"));
    assert!(super::web_static_file_for_request(
        "/health",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .is_none());
    assert!(super::web_static_file_for_request(
        "/health/mesh",
        Some(&root),
        Some(super::ControllerProfile::Native),
    )
    .is_none());
    assert!(super::web_static_file_for_request(
        "/health?probe=1",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relay_share_database_storage_uses_restart_stable_suffix() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let token = uuid::Uuid::new_v4();
    let stored = super::persist_relay_share_database(&state, token, "shares.sqlite", b"fixture")
        .expect("persist relay share database");
    let expected = format!("share-{}.db", token.simple());
    assert_eq!(
        stored.file_name().and_then(|name| name.to_str()),
        Some(expected.as_str())
    );
    assert!(stored.is_file());
    assert!(!state
        .config
        .state_dir
        .join("relay/incoming")
        .join(format!("share-{}.sqlite", token.simple()))
        .exists());
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[test]
fn cancelled_share_scan_stops_before_indexing() {
    let root = std::env::temp_dir().join(format!(
        "slskr-cancelled-share-scan-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("create cancelled scan root");
    fs::write(root.join("not-indexed.flac"), b"fixture").expect("write cancelled scan file");
    let directories = crate::config::parse_share_directories(root.to_str().unwrap())
        .expect("parse cancelled scan directory");
    let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let scan = super::scan_share_dirs_with_cancellation(super::ShareScanRequest {
        directories: &directories,
        follow_symlinks: false,
        include_hidden: false,
        max_files: 100,
        probe_media_attributes: false,
        workers: 1,
        filters: &[],
        cancellation,
    });

    assert!(scan.cancelled);
    assert!(scan.entries.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn graceful_shutdown_cancels_registered_share_rebuild_before_publish() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create share-index persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let before = state.shares.read().await.clone();

    // Keep the production rebuild between cancellation registration and worker
    // startup, so shutdown necessarily overlaps the active rebuild operation.
    let lifecycle_guard = state.share_lifecycle.write().await;
    let scan_state = Arc::clone(&state);
    let mut rebuild = tokio::spawn(async move { super::rebuild_share_index(&scan_state).await });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let registered = state
                .share_scan_cancellation
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_some();
            if registered {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("rebuild registers its shutdown cancellation token");

    super::initiate_graceful_shutdown(&state).await;
    assert!(state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .is_some_and(|cancellation| cancellation.load(std::sync::atomic::Ordering::Acquire)));
    drop(lifecycle_guard);

    let rebuild_result = tokio::time::timeout(Duration::from_secs(2), &mut rebuild)
        .await
        .expect("cancelled rebuild returns")
        .expect("join rebuild task");
    assert_eq!(
        rebuild_result.expect_err("shutdown must cancel the rebuild"),
        super::SHARE_SCAN_CANCELLED_ERROR
    );
    assert_eq!(state.shares.read().await.entries, before.entries);
    let lifecycle = state.share_lifecycle.read().await;
    assert!(!lifecycle.scanning);
    assert!(lifecycle.cancelled);
    drop(lifecycle);
    assert!(db.list_share_files(100, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_durability_snapshots_after_lock_and_rehydrates_in_event_order() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let events_path = super::transfer_events_path(&state.config.state_dir);
    let state_path = super::transfer_state_path(&state.config.state_dir);
    let events_before = fs::read_to_string(&events_path).expect("transfer event header");
    let state_before = fs::read_to_string(&state_path).ok();

    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("restart-peer".to_owned()),
            "Remote/Restart.flac".to_owned(),
            None,
            Some(100),
        );
        transfers.update_status(entry.id, "in_progress", Some(25), None);

        // Queue mutations only snapshot under the lock; no event append or
        // state rewrite is allowed until the guard has been released.
        assert_eq!(
            fs::read_to_string(&events_path).expect("events while locked"),
            events_before
        );
        assert_eq!(fs::read_to_string(&state_path).ok(), state_before);
    }

    super::persist_transfer_durability(&state).await;
    let events_after = fs::read_to_string(&events_path).expect("persisted transfer events");
    assert!(events_after.contains("\tqueued\t"));
    assert!(events_after.contains("\t25\tin_progress\t"));
    assert_eq!(
        events_after.lines().count(),
        events_before.lines().count() + 2
    );
    let persisted = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&state_path).expect("persisted transfer state"),
    )
    .expect("transfer state JSON");
    assert_eq!(persisted["entries"][0]["status"], "in_progress");

    let restarted = super::TransferQueue::new(&state.config);
    assert_eq!(restarted.entries.len(), 1);
    assert_eq!(restarted.entries[0].status, "queued");
    assert_eq!(
        restarted.entries[0].reason.as_deref(),
        Some("resumed after restart")
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_request_name_update_advances_existing_database_revision() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create transfer request-name database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let entry = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("revision-peer".to_owned()),
            "Remote/Revision.flac".to_owned(),
            None,
            Some(100),
        );
        let mut entry = entry;
        entry.updated_at_ms = entry.updated_at_ms.saturating_add(30_000);
        let current = transfers
            .entries
            .iter_mut()
            .find(|current| current.id == entry.id)
            .expect("created transfer remains in queue");
        current.updated_at_ms = entry.updated_at_ms;
        entry
    };
    let initial_revision = entry.updated_at_ms;
    db.insert_transfer_records_with_events(&[(
        super::persisted_transfer_record(&entry),
        super::persisted_transfer_event_record(&entry),
    )])
    .await
    .expect("seed durable transfer snapshot");

    let request_id = entry.request_id.as_deref().expect("download request id");
    let headers = super::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1".parse().expect("loopback test address")),
        ..super::RequestSecurityHeaders::default()
    };
    let response = super::route_dispatch::route_http_request_with_headers(
        "PATCH",
        &format!("/api/v0/downloads/requests/{request_id}/name"),
        None,
        r#"{"name":"renamed request"}"#,
        &state,
        &headers,
    )
    .await
    .expect("rename download request");
    assert_eq!(response.status, "200 OK");

    let updated = db
        .get_transfer(&entry.id.to_string())
        .await
        .expect("read renamed transfer")
        .expect("renamed transfer remains persisted");
    assert_eq!(updated.request_name.as_deref(), Some("renamed request"));
    assert!(
        updated.updated_at_ms > i64::try_from(initial_revision).unwrap(),
        "the persisted transfer revision must advance past its prior value"
    );
    let live = state.transfers.read().await;
    let current = live
        .entries
        .iter()
        .find(|current| current.id == entry.id)
        .expect("renamed transfer remains in memory");
    assert_eq!(current.request_name.as_deref(), Some("renamed request"));
    assert_eq!(
        i64::try_from(current.updated_at_ms).unwrap(),
        updated.updated_at_ms
    );
    drop(live);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn distributed_persistence_worker_commits_the_latest_snapshot() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory distributed database");
    let initial = super::DistributedRuntime::new(Some("local-user")).persistence_snapshot();
    let (snapshot_sender, snapshot_receiver) = tokio::sync::watch::channel(initial);
    let (status_sender, mut status_receiver) =
        tokio::sync::watch::channel(super::DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });
    let worker = tokio::spawn(super::run_distributed_persistence_worker(
        Some(db.clone()),
        snapshot_receiver,
        status_sender,
    ));

    snapshot_sender.send_replace(super::DistributedPersistenceSnapshot {
        revision: 1,
        branch_level: 2,
        branch_root: "intermediate-root".to_owned(),
        parent_username: Some("intermediate-parent".to_owned()),
        children: vec![("intermediate-child".to_owned(), 1)],
    });
    snapshot_sender.send_replace(super::DistributedPersistenceSnapshot {
        revision: 2,
        branch_level: 5,
        branch_root: "latest-root".to_owned(),
        parent_username: Some("latest-parent".to_owned()),
        children: vec![("latest-child".to_owned(), 4)],
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = status_receiver.borrow_and_update().clone();
            if status.revision >= 2 {
                assert!(status.result.is_ok(), "latest snapshot should be durable");
                break;
            }
            status_receiver
                .changed()
                .await
                .expect("persistence worker remains available");
        }
    })
    .await
    .expect("latest distributed snapshot was persisted");

    let (tree_state, children) = db
        .load_distributed_state()
        .await
        .expect("load distributed snapshot");
    assert_eq!(
        tree_state,
        Some((
            5,
            "latest-root".to_owned(),
            Some("latest-parent".to_owned())
        ))
    );
    assert_eq!(children, vec![("latest-child".to_owned(), 4)]);

    drop(snapshot_sender);
    worker.await.expect("persistence worker exits");
    db.close_for_test().await;
}

#[tokio::test]
async fn distributed_runtime_rehydrates_one_persisted_snapshot_after_restart() {
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-runtime-restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create distributed restart directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create distributed restart database");
    let children = vec![("child-a".to_owned(), 2), ("child-b".to_owned(), 4)];
    db.save_distributed_state(6, "branch-root", Some("parent-user"), &children)
        .await
        .expect("persist distributed runtime state");
    db.close_for_test().await;

    let reopened = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen distributed restart database");
    let runtime = RwLock::new(super::DistributedRuntime::new(Some("local-user")));
    super::hydrate_distributed_runtime(&runtime, &reopened)
        .await
        .expect("hydrate distributed runtime");
    let runtime = runtime.read().await;
    assert_eq!(runtime.branch_level, 6);
    assert_eq!(runtime.branch_root, "branch-root");
    assert_eq!(runtime.parent.as_deref(), Some("parent-user"));
    assert_eq!(
        runtime.child_depths,
        children.into_iter().collect::<BTreeMap<_, _>>()
    );
    drop(runtime);
    reopened.close_for_test().await;
    fs::remove_dir_all(root).expect("remove distributed restart directory");
}

#[tokio::test]
async fn transfer_durability_wait_does_not_block_queue_mutations() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("slow-disk-peer".to_owned()),
            "Remote/SlowDisk.flac".to_owned(),
            None,
            Some(100),
        );
    }

    let coordinator = state
        .transfers
        .read()
        .await
        .durability_snapshot()
        .expect("created transfer has a pending durability snapshot")
        .coordinator;
    // Hold the serialization gate to model a slow durability write. The
    // queue lock must be available while that persistence work is delayed.
    let coordinator_guard = coordinator.state.lock().await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let persistence_state = Arc::clone(&state);
    let persistence = tokio::spawn(async move {
        let _ = started_tx.send(());
        super::persist_transfer_durability(&persistence_state).await;
    });
    started_rx.await.expect("persistence task started");
    assert!(
        !persistence.is_finished(),
        "persistence is waiting on the gate"
    );

    let transfers = tokio::time::timeout(Duration::from_secs(1), state.transfers.write())
        .await
        .expect("durability work must not retain the transfer queue lock");
    assert_eq!(transfers.entries.len(), 1);
    drop(transfers);

    drop(coordinator_guard);
    persistence
        .await
        .expect("transfer durability task must finish");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_event_sync_delay_keeps_queue_writable() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let events_path = super::transfer_events_path(&state.config.state_dir);
    {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("delayed-sync-peer".to_owned()),
            "Remote/DelayedSync.flac".to_owned(),
            None,
            Some(100),
        );
    }

    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    super::transfer_state_io::pause_transfer_event_sync(
        events_path.clone(),
        started_tx,
        release_rx,
    );
    let persistence_state = Arc::clone(&state);
    let persistence = tokio::spawn(async move {
        super::persist_transfer_durability(&persistence_state).await;
    });
    tokio::task::spawn_blocking(move || {
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("file-backed append reached the delayed sync point");
    })
    .await
    .expect("sync pause signal task finishes");

    let mut transfers = tokio::time::timeout(Duration::from_secs(1), state.transfers.write())
        .await
        .expect("slow file sync must not retain the transfer queue lock");
    assert_eq!(transfers.entries.len(), 1);
    transfers.update_status(1, "in_progress", Some(25), None);
    drop(transfers);

    release_tx.send(()).expect("release file sync");
    persistence.await.expect("durability flush finishes");
    super::persist_transfer_durability(&state).await;
    let events = fs::read_to_string(events_path).expect("read durable transfer events");
    assert!(events.contains("\tqueued\t"));
    assert!(events.contains("\t25\tin_progress\t"));
    let restarted = super::TransferQueue::new(&state.config);
    assert_eq!(restarted.entries.len(), 1);
    assert_eq!(restarted.entries[0].bytes_transferred, 25);
    assert_eq!(restarted.entries[0].status, "queued");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_durability_survives_killed_writer_process() {
    if let Some(root) = std::env::var_os("SLSKR_RF047_CHILD_DIR") {
        let root = std::path::PathBuf::from(root);
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().expect("state path")),
        );
        for index in 0..32 {
            {
                let mut transfers = state.transfers.write().await;
                let entry = transfers.create(
                    0,
                    Some("crash-peer".to_owned()),
                    format!("Remote/Crash-{index}.flac"),
                    None,
                    Some(100),
                );
                transfers.update_status(entry.id, "in_progress", Some(25), None);
            }
            super::persist_transfer_durability(&state).await;
        }
        fs::write(root.join("ready"), b"durable").expect("announce durable transfer state");
        use std::io::Read;
        let _ = std::io::stdin().read(&mut [0_u8; 1]);
        return;
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "slskr-transfer-crash-restart-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create transfer restart directory");
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "focused_controller_tests::transfer_durability_survives_killed_writer_process",
        ])
        .env("SLSKR_RF047_CHILD_DIR", &root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start transfer writer process");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !root.join("ready").exists() && std::time::Instant::now() < deadline {
        if child.try_wait().expect("check writer process").is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = root.join("ready").exists();
    if !ready {
        let _ = child.kill();
        let _ = child.wait();
        panic!("writer did not publish durable state before timeout");
    }
    child.kill().expect("kill transfer writer without shutdown");
    child.wait().expect("reap killed transfer writer");

    let (restarted, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().expect("state path")),
    );
    let transfers = restarted.transfers.read().await;
    assert_eq!(transfers.entries.len(), 32);
    for entry in &transfers.entries {
        assert_eq!(entry.bytes_transferred, 25);
        assert_eq!(entry.status, "queued");
        assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
    }
    drop(transfers);
    let events = fs::read_to_string(root.join("transfer-events.tsv"))
        .expect("read transfer events after restart");
    assert_eq!(events.matches("\tqueued\t").count(), 32);
    assert_eq!(events.matches("\t25\tin_progress\t").count(), 32);
    fs::remove_dir_all(root).expect("remove transfer restart directory");
}

#[tokio::test]
async fn wishlist_startup_loader_rehydrates_ignored_rules_after_database_reopen() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-wishlist-startup-reload-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create wishlist restart directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create wishlist restart database");
    db.upsert_wishlist_item(&super::persistence::WishlistItemRecord {
        id: "wish-17".to_owned(),
        artist: "Artist".to_owned(),
        title: "Rehydrated Album".to_owned(),
        kind: "Audio".to_owned(),
        filter: "flac".to_owned(),
        enabled: true,
        auto_download: false,
        max_results: 10,
        max_downloads: None,
        last_viewed_at: None,
        last_searched_at: None,
        last_match_count: 0,
        last_visible_hit_count: 0,
        last_hidden_locked_hit_count: 0,
        last_filtered_out_hit_count: 0,
        last_ignored_result_hit_count: 0,
        last_response_count: 0,
        total_search_count: 0,
        total_download_count: 0,
        last_search_id: None,
        lidarr_album_id: None,
        lidarr_track_id: None,
        lidarr_track_count: None,
        lidarr_duration_seconds: None,
        lidarr_release_disambiguation: None,
        added_at: 1,
    })
    .await
    .expect("persist wishlist item");
    db.upsert_wishlist_ignored_result_and_searches(
        &super::persistence::WishlistIgnoredResultRecord {
            id: "ignored-17".to_owned(),
            wishlist_item_id: "wish-17".to_owned(),
            username: "PeerOne".to_owned(),
            directory: "Remote/Album".to_owned(),
            created_at: 2,
        },
        &[],
    )
    .await
    .expect("persist ignored-result rule");
    db.close_for_test().await;

    let reopened = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen wishlist restart database");
    let wishlist = super::load_wishlist_store(Some(&reopened))
        .await
        .expect("load wishlist state through the application startup path");
    reopened.close_for_test().await;

    let (state, mut session_commands) = test_state_with_env(MapEnv::default());
    *state.wishlist.write().await = wishlist;
    let ignored = super::route_http_request(
        "GET",
        "/api/wishlist/wish-17/ignored-results",
        None,
        "",
        &state,
    )
    .await
    .expect("read rehydrated ignored-result rule");
    assert_eq!(ignored.status, "200 OK");
    let ignored_json = serde_json::from_str::<serde_json::Value>(&ignored.body).unwrap();
    assert_eq!(ignored_json[0]["id"], "ignored-17");
    assert_eq!(ignored_json[0]["directory"], "Remote/Album");

    let started =
        super::route_http_request("POST", "/api/wishlist/wish-17/search", None, "", &state)
            .await
            .expect("start search for rehydrated wishlist item");
    assert_eq!(started.status, "202 Accepted");
    let token = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["token"]
        .as_u64()
        .expect("search token");
    assert!(matches!(
        session_commands.try_recv().expect("wishlist search command"),
        super::SessionCommand::Search {
            token: command_token,
            target: super::SearchDispatchTarget::Wishlist,
            ..
        } if u64::from(command_token) == token
    ));

    for filename in ["Remote/Album/Blocked.flac", "Remote/Other/Allowed.flac"] {
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(
                r#"{{"token":{token},"username":"peerone","filename":"{filename}","size":1}}"#
            ),
            &state,
        )
        .await
        .expect("ingest result after wishlist startup hydration");
    }
    let search = state.searches.read().await.records[0].clone();
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].filename, "Remote/Other/Allowed.flac");

    fs::remove_dir_all(root).expect("remove wishlist restart directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn lidarr_rejection_waits_for_search_turn_before_mutating_wishlist() {
    let (state, _session_commands) = test_state_with_env(
        MapEnv::default().with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true"),
    );
    let item = state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .expect("wishlist item");
    let transfer = {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Album/track.flac".to_owned(),
            None,
            Some(8),
            None,
            super::TransferRequestDetails {
                wishlist_item_id: Some(item.id.clone()),
                ..Default::default()
            },
        )
    };
    let result = serde_json::json!({
        "rejectedCandidateCount": 1,
        "rejectedFilenames": []
    });

    let search_turn = state.search_persistence_lock.lock().await;
    let policy_state = Arc::clone(&state);
    let mut policy = tokio::spawn(async move {
        super::transfer_completion::apply_lidarr_rejection_policy(
            &policy_state,
            &transfer,
            "",
            &result,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut policy)
        .await
        .is_err());
    assert!(state
        .wishlist
        .read()
        .await
        .list_ignored_results(&item.id)
        .expect("wishlist item remains available")
        .is_empty());

    drop(search_turn);
    policy
        .await
        .expect("Lidarr policy task completes")
        .expect("apply Lidarr rejection policy");
    assert_eq!(
        state
            .wishlist
            .read()
            .await
            .list_ignored_results(&item.id)
            .expect("wishlist item remains available")
            .len(),
        1
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_ignore_and_search_response_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Album","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();
    let started = super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .expect("start wishlist search");
    assert_eq!(started.status, "202 Accepted");
    let started_json = serde_json::from_str::<serde_json::Value>(&started.body).unwrap();
    let token = started_json["token"].as_u64().expect("search token");
    let search_id = started_json["search_id"]
        .as_str()
        .expect("search id")
        .to_owned();

    // Queue a matching result before the ignore request. The shared turn must
    // let the response persist first, then commit the rule and the suppressed
    // search snapshot together.
    let persistence_turn = state.wishlist_search_persistence_lock.lock().await;
    let response_state = Arc::clone(&state);
    let mut response = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(
                r#"{{"token":{token},"username":"PeerOne","filename":"Remote/Album/Blocked.flac","size":1}}"#
            ),
            &response_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut response)
            .await
            .is_err()
    );

    let ignore_state = Arc::clone(&state);
    let ignore_path = format!("/api/wishlist/{item_id}/ignored-results");
    let mut ignore = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            &ignore_path,
            None,
            r#"{"username":"PeerOne","directory":"Remote/Album"}"#,
            &ignore_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ignore)
        .await
        .is_err());

    drop(persistence_turn);
    let response = response
        .await
        .expect("search response task completes")
        .expect("ingest matching search response");
    assert_eq!(response.status, "200 OK");
    let ignored = ignore
        .await
        .expect("ignored-result task completes")
        .expect("create ignored-result rule");
    assert_eq!(ignored.status, "201 Created");
    assert!(state.searches.read().await.records[0].results.is_empty());
    assert!(db
        .list_search_results(Some(&search_id), 10, 0)
        .await
        .expect("read ordered persisted search results")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_ignore_prevents_a_queued_search_snapshot_from_restoring_results() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist snapshot-order database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Album","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();
    let started = super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .expect("start wishlist search");
    let token = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["token"]
        .as_u64()
        .expect("search token") as u32;
    let search_id = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["search_id"]
        .as_str()
        .expect("search id")
        .to_owned();
    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &format!(
            r#"{{"token":{token},"username":"PeerOne","filename":"Remote/Album/Blocked.flac","size":1}}"#
        ),
        &state,
    )
    .await
    .expect("persist matching search result");

    // Let ignore creation queue first for SQLite. A later status snapshot must
    // re-read the committed in-memory record after the ignore transaction.
    let persistence_turn = state.search_persistence_lock.lock().await;
    let ignore_state = Arc::clone(&state);
    let ignore_path = format!("/api/wishlist/{item_id}/ignored-results");
    let mut ignore = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            &ignore_path,
            None,
            r#"{"username":"PeerOne","directory":"Remote/Album"}"#,
            &ignore_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ignore)
        .await
        .is_err());

    let stale_snapshot = {
        let mut searches = state.searches.write().await;
        let (record, transitioned) = searches.complete(token).expect("complete wishlist search");
        assert!(transitioned);
        record
    };
    let snapshot_state = Arc::clone(&state);
    let mut snapshot_write = tokio::spawn(async move {
        super::persist_search_record(&snapshot_state, &stale_snapshot).await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut snapshot_write)
            .await
            .is_err()
    );

    drop(persistence_turn);
    let ignored = ignore
        .await
        .expect("ignored-result task completes")
        .expect("create ignored-result rule");
    assert_eq!(ignored.status, "201 Created");
    snapshot_write
        .await
        .expect("queued snapshot task completes")
        .expect("persist latest search snapshot");

    let current = state
        .searches
        .read()
        .await
        .get(token)
        .expect("search remains");
    assert_eq!(current.status, "completed");
    assert!(current.results.is_empty());
    assert!(db
        .list_search_results(Some(&search_id), 10, 0)
        .await
        .expect("read suppressed persisted results")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_item_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Original","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();

    let persistence_turn = state.wishlist_search_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/wishlist/{item_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"artist":"Artist","title":"Updated","filter":"flac"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/wishlist/{item_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable =
        super::route_http_request("GET", &format!("/api/wishlist/{item_id}"), None, "", &state)
            .await
            .expect("wishlist read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["title"],
        "Original"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("wishlist update completes")
        .expect("update wishlist item");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("wishlist delete completes")
        .expect("delete wishlist item");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.wishlist.read().await.get_item(&item_id).is_none());
    assert!(!db
        .list_wishlist_items(10, 0)
        .await
        .expect("read final wishlist rows")
        .iter()
        .any(|item| item.id == item_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn library_snapshot_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create library item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/v0/library/items",
        None,
        r#"{"artist":"Artist","title":"Album","kind":""}"#,
        &state,
    )
    .await
    .expect("create library item with a fixable kind");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("library item id")
        .to_owned();

    let persistence_turn = state.library_persistence_lock.lock().await;
    let fix_state = Arc::clone(&state);
    let mut fix = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/library/health/issues/fix",
            None,
            "{}",
            &fix_state,
        )
        .await
    });
    if let Ok(result) = tokio::time::timeout(Duration::from_millis(10), &mut fix).await {
        let response = result
            .expect("library remediation request does not panic")
            .expect("library remediation route handles the request");
        panic!(
            "library remediation bypassed its persistence turn: {} {}",
            response.status, response.body
        );
    }

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/v0/library/items/{item_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/v0/library/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("library read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["kind"],
        ""
    );

    drop(persistence_turn);
    let fixed = fix
        .await
        .expect("library remediation completes")
        .expect("fix library item");
    assert_eq!(fixed.status, "200 OK");
    let deleted = delete
        .await
        .expect("library delete completes")
        .expect("delete library item");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.library.read().await.get(&item_id).is_none());
    assert!(!db
        .list_library_items(10, 0)
        .await
        .expect("read final library rows")
        .iter()
        .any(|item| item.id == item_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn contact_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create contact ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create contact");
    assert_eq!(created.status, "201 Created");
    let contact_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("contact id")
        .to_owned();

    let persistence_turn = state.contact_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/contacts/{contact_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"username":"updated","online":true}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/contacts/{contact_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/contacts/{contact_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("contact read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["username"],
        "friend"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("contact update completes")
        .expect("update contact");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("contact delete completes")
        .expect("delete contact");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.contacts.read().await.get(&contact_id).is_none());
    assert!(!db
        .list_contacts(10, 0)
        .await
        .expect("read final contact rows")
        .iter()
        .any(|contact| contact.id == contact_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn user_note_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create user-note ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/users/notes",
        None,
        r#"{"username":"friend","note":"Original"}"#,
        &state,
    )
    .await
    .expect("create user note");
    assert_eq!(created.status, "201 Created");
    let note_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("user-note id")
        .to_owned();

    let persistence_turn = state.user_note_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/users/notes/{note_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"note":"Updated"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/users/notes/{note_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/users/notes/{note_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("user-note read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["note"],
        "Original"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("user-note update completes")
        .expect("update user note");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("user-note delete completes")
        .expect("delete user note");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.user_notes.read().await.get(&note_id).is_none());
    assert!(!db
        .list_user_notes(10, 0)
        .await
        .expect("read final user-note rows")
        .iter()
        .any(|note| note.id == note_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn interest_mutations_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create interest ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.interest_persistence_lock.lock().await;
    let add_state = Arc::clone(&state);
    let mut add = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/soulseek/interests",
            None,
            r#"{"name":"Jazz"}"#,
            &add_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut add)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let mut delete = tokio::spawn(async move {
        super::route_http_request(
            "DELETE",
            "/api/soulseek/interests/Jazz",
            None,
            "",
            &delete_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let hate_state = Arc::clone(&state);
    let mut hate = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/soulseek/hated-interests",
            None,
            r#"{"name":"Noise"}"#,
            &hate_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut hate)
        .await
        .is_err());

    let liked_read = super::route_http_request("GET", "/api/soulseek/interests", None, "", &state)
        .await
        .expect("liked-interest read remains available while writes wait");
    assert_eq!(liked_read.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&liked_read.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let hated_read =
        super::route_http_request("GET", "/api/soulseek/hated-interests", None, "", &state)
            .await
            .expect("hated-interest read remains available while writes wait");
    assert_eq!(hated_read.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&hated_read.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        add.await
            .expect("interest creation completes")
            .expect("create liked interest")
            .status,
        "201 Created"
    );
    assert_eq!(
        delete
            .await
            .expect("interest deletion completes")
            .expect("delete liked interest")
            .status,
        "200 OK"
    );
    assert_eq!(
        hate.await
            .expect("hated-interest creation completes")
            .expect("create hated interest")
            .status,
        "201 Created"
    );

    let interests = state.interests.read().await;
    assert!(interests.liked.is_empty());
    assert_eq!(interests.hated.len(), 1);
    assert_eq!(interests.hated[0].name, "Noise");
    drop(interests);
    let persisted = db
        .list_interests(10, 0)
        .await
        .expect("read final interest rows");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].kind, "hated");
    assert_eq!(persisted[0].name, "Noise");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn mesh_interest_mutations_roll_back_when_persistence_fails() {
    let post_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create mesh-interest creation database");
    let (post_state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        post_db.clone(),
    );
    post_db.close_for_test().await;
    let headers = super::RequestSecurityHeaders::default();
    let created = super::extended_controller_mutation_response(
        "POST",
        "/api/soulseek/mesh-rendezvous/interest",
        None,
        "",
        &post_state,
        true,
        &headers,
    )
    .await;
    assert_eq!(created.status, "503 Service Unavailable");
    assert!(post_state.interests.read().await.liked.is_empty());

    let delete_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create mesh-interest deletion database");
    let (delete_state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        delete_db.clone(),
    );
    delete_state
        .interests
        .write()
        .await
        .add_liked(super::MESH_RENDEZVOUS_INTEREST_TAG.to_owned())
        .expect("seed mesh interest");
    delete_db.close_for_test().await;
    let deleted = super::extended_controller_mutation_response(
        "DELETE",
        "/api/soulseek/mesh-rendezvous/interest",
        None,
        "",
        &delete_state,
        true,
        &headers,
    )
    .await;
    assert_eq!(deleted.status, "503 Service Unavailable");
    let interests = delete_state.interests.read().await;
    assert_eq!(interests.liked.len(), 1);
    assert_eq!(interests.liked[0].name, super::MESH_RENDEZVOUS_INTEREST_TAG);
}

#[tokio::test]
async fn now_playing_update_and_clear_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create now-playing ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"friend","artist":"Artist","title":"Original"}"#,
        &state,
    )
    .await
    .expect("create now-playing record");
    assert_eq!(created.status, "200 OK");

    let persistence_turn = state.now_playing_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/nowplaying",
            None,
            r#"{"username":"friend","artist":"Artist","title":"Updated"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let clear_state = Arc::clone(&state);
    let mut clear = tokio::spawn(async move {
        super::route_http_request("DELETE", "/api/nowplaying", None, "", &clear_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut clear)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/nowplaying", None, "", &state)
        .await
        .expect("now-playing read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    let records = readable_json["now_playing"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["title"], "Original");

    drop(persistence_turn);
    assert_eq!(
        update
            .await
            .expect("now-playing update completes")
            .expect("update now-playing record")
            .status,
        "200 OK"
    );
    assert_eq!(
        clear
            .await
            .expect("now-playing clear completes")
            .expect("clear now-playing records")
            .status,
        "200 OK"
    );
    assert!(state.now_playing.read().await.records.is_empty());
    assert!(db
        .list_now_playing(10, 0)
        .await
        .expect("read final now-playing rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn webhook_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create webhook ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.com/persistence-order","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let webhook_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("webhook id")
        .to_owned();

    let persistence_turn = state.webhook_persistence_lock.lock().await;
    let patch_state = Arc::clone(&state);
    let patch_path = format!("/api/webhooks/{webhook_id}");
    let mut patch = tokio::spawn(async move {
        super::route_http_request(
            "PATCH",
            &patch_path,
            None,
            r#"{"active":false}"#,
            &patch_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut patch)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/webhooks/{webhook_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/webhooks", None, "", &state)
        .await
        .expect("webhook read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let webhooks_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(webhooks_json["webhooks"][0]["active"], true);

    drop(persistence_turn);
    assert_eq!(
        patch
            .await
            .expect("webhook update completes")
            .expect("patch webhook")
            .status,
        "200 OK"
    );
    assert_eq!(
        delete
            .await
            .expect("webhook deletion completes")
            .expect("delete webhook")
            .status,
        "200 OK"
    );
    assert!(state.webhooks.read().await.get(&webhook_id).is_none());
    assert!(db
        .list_webhooks()
        .await
        .expect("read final webhook rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn security_ban_routes_share_one_persistence_order_across_dispatchers() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create security-ban ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.security_ban_persistence_lock.lock().await;
    let ban_state = Arc::clone(&state);
    let mut ban = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/security/bans/username",
            None,
            r#"{"username":"persistence-order-peer"}"#,
            &ban_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ban)
        .await
        .is_err());

    let unban_state = Arc::clone(&state);
    let mut unban = tokio::spawn(async move {
        super::route_http_request(
            "DELETE",
            "/api/overlay/blocklist/username/persistence-order-peer",
            None,
            "",
            &unban_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut unban)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/security/bans", None, "", &state)
        .await
        .expect("security-ban read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["bans"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        ban.await
            .expect("security ban completes")
            .expect("post security ban")
            .status,
        "200 OK"
    );
    assert_eq!(
        unban
            .await
            .expect("security unban completes")
            .expect("delete overlay blocklist entry")
            .status,
        "200 OK"
    );
    assert!(state.security.read().await.bans.is_empty());
    assert!(db
        .list_security_bans()
        .await
        .expect("read final security-ban rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn overlay_blocklist_mutations_roll_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create overlay blocklist rollback database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    db.close_for_test().await;

    let created = super::route_http_request(
        "POST",
        "/api/overlay/blocklist/username",
        None,
        r#"{"value":"must-not-persist"}"#,
        &state,
    )
    .await
    .expect("failed overlay blocklist ban response");
    assert_eq!(created.status, "503 Service Unavailable");
    assert!(state.security.read().await.bans.is_empty());

    state
        .security
        .write()
        .await
        .ban("username", "must-remain".to_owned())
        .expect("seed overlay blocklist ban");
    let deleted = super::route_http_request(
        "DELETE",
        "/api/overlay/blocklist/username/must-remain",
        None,
        "",
        &state,
    )
    .await
    .expect("failed overlay blocklist unban response");
    assert_eq!(deleted.status, "503 Service Unavailable");
    let security = state.security.read().await;
    assert_eq!(security.bans.len(), 1);
    assert_eq!(security.bans[0].value, "must-remain");
}

#[tokio::test]
async fn message_create_and_conversation_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create message ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.message_persistence_lock.lock().await;
    let create_state = Arc::clone(&state);
    let mut create = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/conversations/friend",
            None,
            r#"{"message":"queued message"}"#,
            &create_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut create)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let mut delete = tokio::spawn(async move {
        super::route_http_request(
            "DELETE",
            "/api/conversations/friend",
            None,
            "",
            &delete_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/conversations/friend", None, "", &state)
        .await
        .expect("conversation read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["messages"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        create
            .await
            .expect("conversation message creation completes")
            .expect("create conversation message")
            .status,
        "200 OK"
    );
    assert_eq!(
        delete
            .await
            .expect("conversation deletion completes")
            .expect("delete conversation")
            .status,
        "200 OK"
    );
    assert!(state.messages.read().await.records.is_empty());
    assert!(db
        .list_messages(10, 0)
        .await
        .expect("read final message rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn room_subscription_join_and_leave_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create room subscription ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.room_persistence_lock.lock().await;
    let join_state = Arc::clone(&state);
    let mut join = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/rooms/joined",
            None,
            r#"{"room":"music"}"#,
            &join_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut join)
        .await
        .is_err());

    let leave_state = Arc::clone(&state);
    let mut leave = tokio::spawn(async move {
        super::route_http_request("DELETE", "/api/rooms/joined/music", None, "", &leave_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut leave)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/rooms/joined", None, "", &state)
        .await
        .expect("room read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(!readable.body.contains("music"));

    drop(persistence_turn);
    assert_eq!(
        join.await
            .expect("room join completes")
            .expect("join room")
            .status,
        "201 Created"
    );
    assert_eq!(
        leave
            .await
            .expect("room leave completes")
            .expect("leave room")
            .status,
        "200 OK"
    );
    assert!(!state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|room| room.name == "music" && room.joined));
    assert!(db
        .list_subscribed_rooms()
        .await
        .expect("read final subscribed rooms")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn browse_request_and_cancel_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create browse ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    state.session.write().await.state = "connected";

    let persistence_turn = state.browse_persistence_lock.lock().await;
    let request_state = Arc::clone(&state);
    let mut request = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/request",
            None,
            "",
            &request_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut request)
            .await
            .is_err()
    );

    let cancel_state = Arc::clone(&state);
    let mut cancel = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/cancel",
            None,
            r#"{"reason":"cancelled"}"#,
            &cancel_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut cancel)
        .await
        .is_err());

    let readable = super::route_http_request("GET", "/api/v0/browse", None, "", &state)
        .await
        .expect("browse read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        request
            .await
            .expect("browse request completes")
            .expect("request browse")
            .status,
        "202 Accepted"
    );
    assert_eq!(
        cancel
            .await
            .expect("browse cancellation completes")
            .expect("cancel browse")
            .status,
        "200 OK"
    );

    assert_eq!(
        state.browse.read().await.get("friend").unwrap().status,
        "cancelled"
    );
    let persisted = db
        .list_browse_records(10, 0)
        .await
        .expect("read final browse rows");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].status, "cancelled");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn user_watch_and_unwatch_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create user projection ordering database");
    let (state, mut session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record = state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .expect("seed watched user");
    super::user_store::persist_user_projection(&state, &record)
        .await
        .expect("persist watched user");

    let persistence_turn = state.user_persistence_lock.lock().await;
    let watch_state = Arc::clone(&state);
    let mut watch = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/users/watch",
            None,
            r#"{"username":"friend"}"#,
            &watch_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut watch)
        .await
        .is_err());

    let unwatch_state = Arc::clone(&state);
    let mut unwatch = tokio::spawn(async move {
        super::route_http_request(
            "DELETE",
            "/api/v0/users/friend/watch",
            None,
            "",
            &unwatch_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut unwatch)
            .await
            .is_err()
    );

    let readable = super::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("user read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(readable.body.contains("\"watched\":true"));

    drop(persistence_turn);
    assert_eq!(
        watch
            .await
            .expect("user watch completes")
            .expect("watch user")
            .status,
        "201 Created"
    );
    assert_eq!(
        unwatch
            .await
            .expect("user unwatch completes")
            .expect("unwatch user")
            .status,
        "200 OK"
    );
    assert_eq!(
        session_commands.try_recv().expect("watch session command"),
        super::SessionCommand::WatchUser("friend".to_owned())
    );
    assert_eq!(
        session_commands
            .try_recv()
            .expect("unwatch session command"),
        super::SessionCommand::UnwatchUser("friend".to_owned())
    );
    assert!(!state.users.read().await.records[0].watched);
    let persisted = db
        .list_user_projections(10, 0)
        .await
        .expect("read final user projection");
    assert_eq!(persisted.len(), 1);
    assert!(!persisted[0].watched);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn event_api_and_background_writes_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create event ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.event_persistence_lock.lock().await;
    let api_state = Arc::clone(&state);
    let mut api_event = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""queued""#,
            &api_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut api_event)
            .await
            .is_err()
    );

    let background_state = Arc::clone(&state);
    let mut background_event = tokio::spawn(async move {
        super::record_event(&background_state, "background.event", "resource", None).await;
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut background_event)
            .await
            .is_err()
    );

    let readable = super::route_http_request("GET", "/api/v0/events/records", None, "", &state)
        .await
        .expect("event read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(readable_json["count"], 0);
    assert!(readable_json["entries"].as_array().unwrap().is_empty());

    drop(persistence_turn);
    assert_eq!(
        api_event
            .await
            .expect("API event completes")
            .expect("inject event")
            .status,
        "201 Created"
    );
    background_event
        .await
        .expect("background event persistence completes");

    let memory = state.events.read().await;
    assert_eq!(memory.records.len(), 2);
    assert_eq!(memory.records[0].id, 1);
    assert_eq!(memory.records[0].kind, "Noop");
    assert_eq!(memory.records[1].id, 2);
    assert_eq!(memory.records[1].kind, "background.event");
    drop(memory);
    let persisted = db.list_events(10, 0).await.expect("read final event rows");
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].id, 1);
    assert_eq!(persisted[0].kind, "Noop");
    assert_eq!(persisted[1].id, 2);
    assert_eq!(persisted[1].kind, "background.event");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn oauth_state_consumption_is_ordered_and_keeps_reads_available() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create OAuth state ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let token = "queued-oauth-state".to_owned();
    let now = super::unix_timestamp();
    let record = super::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://localhost/callback".to_owned(),
        code_verifier: Some("test-verifier".to_owned()),
        created_at: now,
        expires_at: now.saturating_add(600),
    };
    state
        .oauth_states
        .write()
        .await
        .records
        .insert(token.clone(), record.clone());
    super::oauth_state::persist_oauth_state_checked(&state, &token, &record)
        .await
        .expect("persist OAuth state");

    let persistence_turn = state.oauth_persistence_lock.lock().await;
    let first_state = Arc::clone(&state);
    let mut first = tokio::spawn(async move {
        super::oauth_state::consume_oauth_state(&first_state, "spotify", &token).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut first)
        .await
        .is_err());

    let second_state = Arc::clone(&state);
    let token = "queued-oauth-state".to_owned();
    let mut second = tokio::spawn(async move {
        super::oauth_state::consume_oauth_state(&second_state, "spotify", &token).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut second)
        .await
        .is_err());

    let readable = state.oauth_states.read().await;
    assert!(readable.records.contains_key("queued-oauth-state"));
    drop(readable);
    drop(persistence_turn);

    assert_eq!(
        first
            .await
            .expect("first OAuth callback completes")
            .expect("consume persisted OAuth state"),
        Some(record)
    );
    assert!(second
        .await
        .expect("second OAuth callback completes")
        .expect("inspect already consumed OAuth state")
        .is_none());
    assert!(!state
        .oauth_states
        .read()
        .await
        .records
        .contains_key("queued-oauth-state"));
    assert!(db
        .list_oauth_states(i64::try_from(now).unwrap_or(i64::MAX), 10, 0)
        .await
        .expect("read final persisted OAuth state")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn compatibility_message_ack_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create compatibility message database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record =
        state
            .messages
            .write()
            .await
            .add("friend".to_owned(), "inbound", "hello".to_owned());
    let path = format!("/api/conversations/friend/{}", record.id);
    db.close_for_test().await;

    let response = super::extended_controller_mutation_response(
        "PUT",
        &path,
        None,
        "",
        &state,
        true,
        &super::RequestSecurityHeaders::default(),
    )
    .await;
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(!state.messages.read().await.records[0].acknowledged);
}

#[tokio::test]
async fn compatibility_message_ack_wrong_username_does_not_mutate_message() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create compatibility message database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record =
        state
            .messages
            .write()
            .await
            .add("friend".to_owned(), "inbound", "hello".to_owned());
    let path = format!("/api/conversations/stranger/{}", record.id);

    let response = super::extended_controller_mutation_response(
        "PUT",
        &path,
        None,
        "",
        &state,
        true,
        &super::RequestSecurityHeaders::default(),
    )
    .await;

    assert_eq!(response.status, "404 Not Found");
    assert!(!state.messages.read().await.records[0].acknowledged);
}

#[tokio::test]
async fn managed_shutdown_persists_final_distributed_snapshot_after_worker_stops() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-shutdown-flush-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create distributed shutdown directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create distributed shutdown database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let (worker_started_tx, worker_started_rx) = tokio::sync::oneshot::channel();
    let (worker_stopped_tx, worker_stopped_rx) = tokio::sync::oneshot::channel::<()>();
    state.spawn_managed_task(async move {
        let mut worker_stopped_tx = Some(worker_stopped_tx);
        worker_started_tx
            .send(())
            .expect("observe managed shutdown worker startup");
        std::future::pending::<()>().await;
        drop(worker_stopped_tx.take());
    });
    worker_started_rx
        .await
        .expect("managed shutdown worker starts before shutdown");

    // Hold the runtime lock while shutdown aborts and joins managed workers.
    // The final snapshot read must wait for this lock and include the latest
    // revision published while shutdown is in progress.
    let mut runtime = state.distributed_network.write().await;
    let shutdown_state = Arc::clone(&state);
    let shutdown = tokio::spawn(async move { shutdown_state.shutdown_managed_tasks().await });
    assert!(
        tokio::time::timeout(Duration::from_secs(1), worker_stopped_rx)
            .await
            .expect("shutdown drops the managed worker")
            .is_err()
    );
    runtime.branch_level = 9;
    runtime.branch_root = "shutdown-root".to_owned();
    runtime.parent = Some("shutdown-parent".to_owned());
    runtime.child_depths = [("shutdown-child".to_owned(), 7)].into_iter().collect();
    runtime.persistence_revision = runtime.persistence_revision.saturating_add(1);
    let snapshot = runtime.persistence_snapshot();
    state
        .distributed_persistence_snapshots
        .send_replace(snapshot);
    drop(runtime);

    shutdown
        .await
        .expect("managed shutdown and final persistence join");
    let (tree_state, children) = db
        .load_distributed_state()
        .await
        .expect("load final distributed shutdown snapshot");
    assert_eq!(
        tree_state,
        Some((
            9,
            "shutdown-root".to_owned(),
            Some("shutdown-parent".to_owned())
        ))
    );
    assert_eq!(children, vec![("shutdown-child".to_owned(), 7)]);

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove distributed shutdown directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_mutations_leave_pod_reads_available_while_waiting_for_channel_store() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for pod_id in ["pod-lock-put", "pod-lock-delete"] {
        let create = serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": "Before",
                "isPublic": true,
                "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
            }
        })
        .to_string();
        let response = super::route_http_request("POST", "/api/v0/pods", None, &create, &state)
            .await
            .expect("create pod for lock-order regression");
        assert_eq!(response.status, "201 Created");
    }

    let update_path = "/api/v0/pods/pod-lock-put";
    let update_body = serde_json::json!({
        "pod": {
            "podId": "pod-lock-put",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let channel_guard = state.pod_channels.write().await;
    let update = super::route_http_request("PUT", update_path, None, &update_body, &state);
    tokio::pin!(update);
    tokio::select! {
        biased;
        _ = &mut update => panic!("pod update should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }
    let readable = tokio::time::timeout(
        Duration::from_secs(1),
        super::route_http_request("GET", update_path, None, "", &state),
    )
    .await
    .expect("pod read should not wait behind a channel-store waiter")
    .expect("read pod during update wait");
    assert_eq!(readable.status, "200 OK");
    drop(channel_guard);
    let updated = tokio::time::timeout(Duration::from_secs(1), &mut update)
        .await
        .expect("pod update should finish after the channel store is released")
        .expect("update pod after channel store is released");
    assert_eq!(updated.status, "200 OK");

    let delete_path = "/api/v0/pods/pod-lock-delete";
    let channel_guard = state.pod_channels.write().await;
    let delete = super::route_http_request("DELETE", delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("pod delete should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }
    let readable = tokio::time::timeout(
        Duration::from_secs(1),
        super::route_http_request("GET", delete_path, None, "", &state),
    )
    .await
    .expect("pod read should not wait behind a channel-store waiter")
    .expect("read pod during delete wait");
    assert_eq!(readable.status, "200 OK");
    drop(channel_guard);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("pod delete should finish after the channel store is released")
        .expect("delete pod after channel store is released");
    assert_eq!(deleted.status, "204 No Content");

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_membership_acceptance_keeps_pending_reads_available_while_room_store_waits() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state
        .rooms
        .write()
        .await
        .join("pod:workflow-lock".to_owned())
        .expect("create room for membership workflow lock regression");
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow-lock",
        "owner-peer",
        "owner".to_owned(),
    );

    let join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"applicant"}"#,
        &state,
    )
    .await
    .expect("create pending join request");
    assert_eq!(join.status, "200 OK");

    let room_guard = state.rooms.write().await;
    let acceptance = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    );
    tokio::pin!(acceptance);
    tokio::select! {
        biased;
        _ = &mut acceptance => panic!("membership acceptance should wait for the held room store"),
        _ = tokio::task::yield_now() => {}
    }

    let pending = tokio::time::timeout(
        Duration::from_secs(1),
        super::route_http_request(
            "GET",
            "/api/v0/podcore/membership/join/pending/pod%3Aworkflow-lock",
            None,
            "",
            &state,
        ),
    )
    .await
    .expect("pending request read must remain available while acceptance waits")
    .expect("read pending join request");
    assert_eq!(pending.status, "200 OK");
    assert!(pending.body.contains("applicant"));

    drop(room_guard);
    let accepted = tokio::time::timeout(Duration::from_secs(1), &mut acceptance)
        .await
        .expect("acceptance should finish once the room store is released")
        .expect("accept pending join request");
    assert_eq!(accepted.status, "200 OK");
    assert!(state
        .pod_membership_workflow
        .read()
        .await
        .pending_joins("pod:workflow-lock")
        .is_empty());
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == "pod:workflow-lock")
        .is_some_and(|room| room.members.iter().any(|member| member == "applicant")));

    let second_join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"second-applicant"}"#,
        &state,
    )
    .await
    .expect("create second pending join request");
    assert_eq!(second_join.status, "200 OK");
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow-lock",
        "owner-peer",
        "owner".to_owned(),
    );

    let room_guard = state.rooms.write().await;
    let revoked_acceptance = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"second-applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    );
    tokio::pin!(revoked_acceptance);
    tokio::select! {
        biased;
        _ = &mut revoked_acceptance => panic!("membership acceptance should wait for the held room store"),
        _ = tokio::task::yield_now() => {}
    }
    state
        .pod_membership_workflow
        .write()
        .await
        .remove_role("pod:workflow-lock", "owner-peer");
    drop(room_guard);
    let rejected = tokio::time::timeout(Duration::from_secs(1), &mut revoked_acceptance)
        .await
        .expect("revoked acceptance should finish after the room store is released")
        .expect("revoked acceptance response");
    assert_eq!(rejected.status, "400 Bad Request");
    assert_eq!(
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:workflow-lock")
            .len(),
        1
    );
    assert!(!state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == "pod:workflow-lock")
        .is_some_and(|room| room
            .members
            .iter()
            .any(|member| member == "second-applicant")));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_channel_append_queued_after_channel_removal_is_rejected() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "pod": {
            "podId": "pod:append-race",
            "name": "Before",
            "isPublic": true,
            "channels": [
                {"channelId": "general", "kind": 0, "name": "General"},
                {"channelId": "removed", "kind": 1, "name": "Removed"}
            ]
        }
    })
    .to_string();
    let created = super::route_http_request("POST", "/api/v0/pods", None, &create, &state)
        .await
        .expect("create pod for queued append regression");
    assert_eq!(created.status, "201 Created");

    let channel_guard = state.pod_channels.write().await;
    let update = serde_json::json!({
        "pod": {
            "podId": "pod:append-race",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let update =
        super::route_http_request("PUT", "/api/v0/pods/pod:append-race", None, &update, &state);
    tokio::pin!(update);
    tokio::select! {
        biased;
        _ = &mut update => panic!("pod update should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }

    let append_body = serde_json::json!({
        "senderPeerId": "tester",
        "body": "must not outlive the removed channel"
    })
    .to_string();
    let append = super::route_http_request(
        "POST",
        "/api/v0/pods/pod:append-race/channels/removed/messages",
        None,
        &append_body,
        &state,
    );
    tokio::pin!(append);
    tokio::select! {
        biased;
        _ = &mut append => panic!("pod message append should queue behind the update"),
        _ = tokio::task::yield_now() => {}
    }

    drop(channel_guard);
    let updated = tokio::time::timeout(Duration::from_secs(1), &mut update)
        .await
        .expect("pod update should complete before queued append")
        .expect("update pod after channel store is released");
    assert_eq!(updated.status, "200 OK");
    let appended = tokio::time::timeout(Duration::from_secs(1), &mut append)
        .await
        .expect("queued append should finish after the update")
        .expect("append route should return a response");
    assert_eq!(appended.status, "404 Not Found");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod:append-race", "removed", None)
        .is_empty());
    let reloaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
        .expect("reload channel state after queued append");
    assert!(reloaded.list("pod:append-race", "removed", None).is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_update_restores_parent_when_channel_cleanup_fails() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "pod": {
            "podId": "pod:cleanup-failure",
            "name": "Before",
            "isPublic": true,
            "channels": [
                {"channelId": "general", "kind": 0, "name": "General"},
                {"channelId": "removed", "kind": 1, "name": "Removed"}
            ]
        }
    })
    .to_string();
    let created = super::route_http_request("POST", "/api/v0/pods", None, &create, &state)
        .await
        .expect("create pod for rollback regression");
    assert_eq!(created.status, "201 Created");
    state
        .pod_channels
        .write()
        .await
        .append(
            "pod:cleanup-failure".to_owned(),
            "removed".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            1,
        )
        .expect("persist message in channel that will be removed");

    let channel_path = state.config.state_dir.join("pod-channel-messages.json");
    let preserved_channel_path = state
        .config
        .state_dir
        .join("pod-channel-messages.json.saved");
    fs::rename(&channel_path, &preserved_channel_path)
        .expect("move channel file before forcing cleanup failure");
    fs::create_dir(&channel_path).expect("make channel state path unwritable");
    let update = serde_json::json!({
        "pod": {
            "podId": "pod:cleanup-failure",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let response = super::route_http_request(
        "PUT",
        "/api/v0/pods/pod:cleanup-failure",
        None,
        &update,
        &state,
    )
    .await
    .expect("failed channel cleanup response");
    assert_eq!(response.status, "500 Internal Server Error");
    fs::remove_dir(&channel_path).expect("remove directory that blocked channel persistence");
    fs::rename(&preserved_channel_path, &channel_path)
        .expect("restore channel file after rollback regression");

    let pod = state
        .pods
        .read()
        .await
        .get("pod:cleanup-failure")
        .expect("pod remains in memory after channel cleanup failure");
    assert_eq!(pod.name, "Before");
    assert_eq!(pod.channels.len(), 2);
    assert_eq!(
        state
            .pod_channels
            .read()
            .await
            .list("pod:cleanup-failure", "removed", None)
            .len(),
        1
    );
    let reloaded =
        super::pods::PodStore::load(&state.config.state_dir).expect("reload restored parent state");
    assert_eq!(
        reloaded
            .get("pod:cleanup-failure")
            .expect("persisted pod remains after rollback")
            .channels
            .len(),
        2
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[test]
fn pod_startup_prunes_channel_messages_left_by_interrupted_parent_commit() {
    let root = std::env::temp_dir().join(format!(
        "slskr-pod-cross-file-recovery-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create pod recovery state directory");
    let mut pods = super::pods::PodStore::empty(&root);
    let mut channels = super::pod_channels::PodChannelStore::empty(&root);

    let mut update_pod = serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
        "podId": "pod:recovery-update",
        "name": "Before update",
        "channels": [
            {"channelId": "general", "kind": 0, "name": "General"},
            {"channelId": "removed", "kind": 1, "name": "Removed"}
        ]
    }))
    .expect("deserialize update recovery pod");
    pods.create(update_pod.clone(), "owner".to_owned())
        .expect("create update recovery pod");
    channels
        .append(
            "pod:recovery-update".to_owned(),
            "removed".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            1,
        )
        .expect("persist channel message removed by update");
    update_pod
        .channels
        .retain(|channel| channel.channel_id == "general");
    pods.update("pod:recovery-update", update_pod)
        .expect("persist parent update before channel cleanup");

    let deleted_pod = serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
        "podId": "pod:recovery-delete",
        "name": "Deleted pod",
        "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
    }))
    .expect("deserialize delete recovery pod");
    pods.create(deleted_pod, "owner".to_owned())
        .expect("create delete recovery pod");
    channels
        .append(
            "pod:recovery-delete".to_owned(),
            "general".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            2,
        )
        .expect("persist channel message removed by pod deletion");
    pods.delete("pod:recovery-delete")
        .expect("persist parent deletion before channel cleanup");
    drop(channels);
    drop(pods);

    let (channels, pods) = super::daemon_serve::load_pod_stores(&root, false)
        .expect("recover pod/channel state during startup");
    assert_eq!(
        pods.get("pod:recovery-update")
            .expect("updated pod survives recovery")
            .channels
            .len(),
        1
    );
    assert!(pods.get("pod:recovery-delete").is_none());
    assert!(channels
        .list("pod:recovery-update", "removed", None)
        .is_empty());
    assert!(channels
        .list("pod:recovery-delete", "general", None)
        .is_empty());
    drop(channels);
    drop(pods);

    let persisted_channels =
        super::pod_channels::PodChannelStore::load(&root).expect("reload repaired channel store");
    assert!(persisted_channels
        .list("pod:recovery-update", "removed", None)
        .is_empty());
    assert!(persisted_channels
        .list("pod:recovery-delete", "general", None)
        .is_empty());
    fs::remove_dir_all(root).expect("remove pod recovery state directory");
}

#[tokio::test]
async fn share_grant_creation_rechecks_collection_after_waiting_for_grant_store() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-grant-parent-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create share grant parent directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create share grant parent database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Concurrent grant collection"}"#,
        &state,
    )
    .await
    .expect("create persisted collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let grant_store = state.share_grants.write().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = super::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete should wait for the held grant store"),
        _ = tokio::task::yield_now() => {}
    }

    let grant_body = serde_json::json!({
        "collection_id": collection_id,
        "username": "recipient",
        "permissions": "read"
    })
    .to_string();
    let create_grant =
        super::route_http_request("POST", "/api/v0/share-grants", None, &grant_body, &state);
    tokio::pin!(create_grant);
    tokio::select! {
        biased;
        _ = &mut create_grant => panic!("share grant creation should wait for the held grant store"),
        _ = tokio::task::yield_now() => {}
    }

    drop(grant_store);
    let (deleted, created) = tokio::join!(&mut delete, &mut create_grant);
    let deleted = deleted.expect("delete collection after grant-store release");
    let created = created.expect("finish grant creation after collection delete");
    assert_eq!(deleted.status, "204 No Content");
    assert_eq!(created.status, "404 Not Found");
    assert!(state.share_grants.read().await.records.is_empty());
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("list persisted collections")
        .is_empty());
    assert!(db
        .list_share_grants(100, 0)
        .await
        .expect("list persisted grants")
        .is_empty());

    let orphan = super::persistence::ShareGrantRecord {
        id: "orphan-grant".to_owned(),
        collection_id,
        username: "recipient".to_owned(),
        shared_at: 1,
        permissions: "read".to_owned(),
    };
    assert!(db.upsert_share_grant(&orphan).await.is_err());
    assert!(db
        .list_share_grants(100, 0)
        .await
        .expect("confirm rejected orphan grant")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove share grant parent directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn stale_collection_snapshot_cannot_resurrect_deleted_parent() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-parent-consistency-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection parent directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection parent database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let created = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Persisted collection"}"#,
        &state,
    )
    .await
    .expect("create collection through production route");
    assert_eq!(created.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();
    let stale_record = state
        .collections
        .read()
        .await
        .get(&collection_id)
        .expect("created collection snapshot");

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection through production route");
    assert_eq!(deleted.status, "204 No Content");
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("confirm collection deletion")
        .is_empty());

    assert!(
        super::collection_store::persist_collection_checked(&state, &stale_record)
            .await
            .is_err()
    );
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("confirm stale snapshot did not recreate collection")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection parent directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_delete_waits_for_persistence_turn_without_blocking_reads() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-persistence-order-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection persistence directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let created = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Persistence order"}"#,
        &state,
    )
    .await
    .expect("create collection");
    assert_eq!(created.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let persistence_turn = state.collection_grant_persistence_lock.lock().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = super::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete must wait for its persistence turn"),
        _ = tokio::task::yield_now() => {}
    }

    let readable = super::route_http_request("GET", &delete_path, None, "", &state)
        .await
        .expect("read collection while delete waits for persistence turn");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(db.list_collections(100, 0).await.unwrap().len(), 1);

    drop(persistence_turn);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("delete completes after persistence turn is released")
        .expect("delete collection");
    assert_eq!(deleted.status, "204 No Content");
    assert!(db.list_collections(100, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection persistence directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn share_group_delete_precedes_queued_member_add_without_blocking_reads() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create share-group persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let created = super::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted peers"}"#,
        &state,
    )
    .await
    .expect("create share group");
    assert_eq!(created.status, "201 Created");
    let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("share-group id")
        .to_owned();
    let member_path = format!("/api/sharegroups/{group_id}/members");
    let member = super::route_http_request(
        "POST",
        &member_path,
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("add initial share-group member");
    assert_eq!(member.status, "201 Created");

    let persistence_turn = state.share_group_persistence_lock.lock().await;
    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/sharegroups/{group_id}");
    let delete_task_path = delete_path.clone();
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_task_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let add_state = Arc::clone(&state);
    let add_path = member_path.clone();
    let mut add = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            &add_path,
            None,
            r#"{"username":"late"}"#,
            &add_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut add)
        .await
        .is_err());

    let readable = super::route_http_request("GET", &delete_path, None, "", &state)
        .await
        .expect("read share group while mutations wait for persistence turn");
    assert_eq!(readable.status, "200 OK");
    let readable_members = super::route_http_request("GET", &member_path, None, "", &state)
        .await
        .expect("read share-group members while mutations wait");
    assert_eq!(readable_members.status, "200 OK");
    assert!(readable_members.body.contains("friend"));
    assert_eq!(db.list_share_groups(10, 0).await.unwrap().len(), 1);

    drop(persistence_turn);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("delete completes after persistence turn is released")
        .expect("join delete route")
        .expect("delete response");
    assert_eq!(deleted.status, "200 OK");
    let added = tokio::time::timeout(Duration::from_secs(1), &mut add)
        .await
        .expect("queued member request completes after delete")
        .expect("join member route")
        .expect("member response");
    assert_eq!(added.status, "404 Not Found");
    assert!(db.list_share_groups(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_group_members(10, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_delete_precedes_queued_share_token_creation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-token-order-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection token-order directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection token-order database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Token order"}"#,
        &state,
    )
    .await
    .expect("create collection for token-order regression");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();
    let grant = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        &serde_json::json!({
            "collection_id": collection_id,
            "username": "recipient",
            "permissions": "read,stream"
        })
        .to_string(),
        &state,
    )
    .await
    .expect("create grant for token-order regression");
    assert_eq!(grant.status, "201 Created");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .expect("grant id")
        .to_owned();

    let persistence_turn = state.collection_grant_persistence_lock.lock().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = super::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete must wait for persistence turn"),
        _ = tokio::task::yield_now() => {}
    }
    let token_path = format!("/api/v0/share-grants/{grant_id}/token");
    let create_token = super::route_http_request("POST", &token_path, None, "", &state);
    tokio::pin!(create_token);
    tokio::select! {
        biased;
        _ = &mut create_token => panic!("share-token creation must wait behind collection delete"),
        _ = tokio::task::yield_now() => {}
    }

    drop(persistence_turn);
    let (deleted, token) = tokio::join!(&mut delete, &mut create_token);
    let deleted = deleted.expect("delete collection before queued token request");
    let token = token.expect("finish queued token request");
    assert_eq!(deleted.status, "204 No Content");
    assert_eq!(token.status, "404 Not Found");
    assert!(state.share_access_tokens.read().await.records.is_empty());
    assert!(db
        .list_share_access_tokens(0, 100, 0)
        .await
        .expect("list persisted tokens after collection delete")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection token-order directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn spotify_disconnect_rejects_an_in_flight_connection_commit() {
    use std::sync::atomic::Ordering;

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let stale_generation = state.spotify_connection_generation.load(Ordering::Acquire);
    let stale_connection = super::SpotifyConnectionStore {
        access_token: "stale-access-token".to_owned(),
        refresh_token: "stale-refresh-token".to_owned(),
        scope: "user-library-read".to_owned(),
        expires_at: i64::try_from(super::unix_timestamp())
            .unwrap_or(i64::MAX)
            .saturating_add(3_600),
        display_name: "Stale account".to_owned(),
        spotify_user_id: "stale-user".to_owned(),
    };

    super::disconnect_spotify_connection(&state)
        .await
        .expect("disconnect Spotify connection");
    assert!(!super::persist_spotify_connection_if_current(
        &state,
        stale_generation,
        &stale_connection,
    )
    .await
    .expect("reject stale Spotify connection commit"));

    assert_eq!(
        *state.spotify_connection.read().await,
        super::SpotifyConnectionStore::default()
    );
    assert!(!super::spotify_connection_path(&state.config.state_dir).exists());
    fs::remove_dir_all(&state.config.state_dir).expect("remove Spotify lifecycle test state");
}

#[tokio::test]
async fn server_state_response_releases_session_lock_while_credentials_are_queued() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for (method, session_state, expected_status) in [
        ("GET", "connected", "200 OK"),
        ("POST", "connected", "202 Accepted"),
        ("POST", "disconnected", "202 Accepted"),
        ("DELETE", "disconnected", "202 Accepted"),
    ] {
        state.session.write().await.state = session_state;
        let held_credentials = state.runtime_credentials.write().await;
        let mut request = Box::pin(super::route_http_request(
            method,
            "/api/server",
            None,
            "",
            &state,
        ));

        let waiting_on_credentials = std::future::poll_fn(|context| {
            Poll::Ready(request.as_mut().poll(context).is_pending())
        })
        .await;
        assert!(
            waiting_on_credentials,
            "{method} should wait for credential configuration"
        );
        assert!(
            state.session.try_write().is_ok(),
            "{method} waiting for credentials must not retain the session lock"
        );

        drop(held_credentials);
        let response = request.await.expect("server state response");
        assert_eq!(response.status, expected_status);
    }
    fs::remove_dir_all(&state.config.state_dir).expect("remove session lock test state");
}

#[tokio::test(flavor = "current_thread")]
async fn transfer_batch_path_preparation_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let mut preparation = Box::pin(super::prepare_download_batch_transfer(
        &state,
        0,
        "peer",
        "Music/A.flac",
        42,
        batch_id,
        None,
    ));
    let waiting_on_destination = std::future::poll_fn(|context| {
        Poll::Ready(preparation.as_mut().poll(context).is_pending())
    })
    .await;
    assert!(
        waiting_on_destination,
        "path resolution should wait for destinations"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "batch path preparation must not retain the transfer lock"
    );

    drop(held_destinations);
    let prepared = preparation.await.expect("focused batch path preparation");
    assert_eq!(prepared.filename, "Music/A.flac");
    assert_eq!(prepared.size, 42);
    assert!(prepared.local_path.is_some());
    {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Music/A.flac".to_owned(),
            None,
            None,
            None,
            super::TransferRequestDetails::default(),
        );
    }
    let (staged, failures) =
        super::commit_prepared_download_batch_transfers(&state, "peer", batch_id, vec![prepared])
            .await;
    assert!(staged.is_empty());
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].1["filename"], "Music/A.flac");
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove transfer batch lock test state");
}

#[tokio::test(flavor = "current_thread")]
async fn legacy_transfer_array_path_resolution_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let body = r#"{"username":"peer","files":[{"filename":"Music/A.flac","size":42}]}"#;
    let mut request = Box::pin(super::route_http_request(
        "POST",
        "/api/transfers",
        None,
        body,
        &state,
    ));
    let waiting_on_destination =
        std::future::poll_fn(|context| Poll::Ready(request.as_mut().poll(context).is_pending()))
            .await;
    assert!(
        waiting_on_destination,
        "legacy array enqueue should wait for destination resolution"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "legacy path resolution must not retain the transfer lock"
    );

    drop(held_destinations);
    let response = request.await.expect("legacy array enqueue response");
    assert_eq!(response.status, "200 OK");
    let response: serde_json::Value =
        serde_json::from_str(&response.body).expect("legacy enqueue response JSON");
    assert_eq!(response["queued"], 1);
    assert_eq!(response["transfers"].as_array().unwrap().len(), 1);
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove legacy transfer lock test state");
}

#[tokio::test(flavor = "current_thread")]
async fn legacy_per_user_transfer_path_resolution_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let body = r#"{"files":[{"filename":"Music/A.flac","size":42}]}"#;
    let mut request = Box::pin(super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        body,
        &state,
    ));
    let waiting_on_destination =
        std::future::poll_fn(|context| Poll::Ready(request.as_mut().poll(context).is_pending()))
            .await;
    assert!(
        waiting_on_destination,
        "per-user enqueue should wait for destination resolution"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "per-user path resolution must not retain the transfer lock"
    );

    drop(held_destinations);
    let response = request.await.expect("per-user enqueue response");
    assert_eq!(response.status, "200 OK");
    let response: serde_json::Value =
        serde_json::from_str(&response.body).expect("per-user enqueue response JSON");
    assert_eq!(response["queued"], 1);
    assert_eq!(response["transfers"].as_array().unwrap().len(), 1);
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove per-user transfer lock test state");
}
