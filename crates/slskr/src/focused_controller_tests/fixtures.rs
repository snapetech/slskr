pub(super) use std::{
    collections::BTreeMap,
    fs,
    sync::{Arc, RwLock as StdRwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(super) use base64::Engine;
pub(super) use tokio::sync::{mpsc, RwLock};

pub(super) use crate::config::{ConfigEnv, FileConfig};
pub(super) use slskr_client::protocol::peer::FileEntry;

#[derive(Clone, Default)]
pub(super) struct MapEnv {
    values: BTreeMap<String, String>,
}

impl MapEnv {
    pub(super) fn with(mut self, name: &str, value: &str) -> Self {
        self.values.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl ConfigEnv for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

pub(super) fn test_state_with_env(
    extra_env: MapEnv,
) -> (Arc<crate::AppState>, mpsc::Receiver<crate::SessionCommand>) {
    test_state_with_env_and_db(extra_env, None)
}

pub(super) fn test_state_with_db(
    extra_env: MapEnv,
    db: crate::persistence::DatabaseManager,
) -> (Arc<crate::AppState>, mpsc::Receiver<crate::SessionCommand>) {
    test_state_with_env_and_db(extra_env, Some(db))
}

pub(super) fn test_state_with_env_and_db(
    extra_env: MapEnv,
    db: Option<crate::persistence::DatabaseManager>,
) -> (Arc<crate::AppState>, mpsc::Receiver<crate::SessionCommand>) {
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
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env)
        .expect("focused test config");
    let share_index = crate::build_share_index(&config);
    let share_lifecycle = crate::ShareLifecycleState::from_snapshot(&share_index);
    let (sender, receiver) = mpsc::channel(8);
    let (event_tx, _) = tokio::sync::broadcast::channel(crate::EVENT_HISTORY_LIMIT);
    let rate_limiter = crate::rate_limit::RateLimiter::new(crate::rate_limit::RateLimitConfig {
        max_requests_anonymous: 1000,
        max_requests_authenticated: 5000,
        window_seconds: 60,
        enabled: true,
    });
    let distributed_runtime = crate::DistributedRuntime::new(config.username.as_deref());
    let (distributed_persistence_snapshots, distributed_persistence_receiver) =
        tokio::sync::watch::channel(distributed_runtime.persistence_snapshot());
    let (distributed_persistence_status_sender, distributed_persistence_status) =
        tokio::sync::watch::channel(crate::DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });

    let state = Arc::new(crate::AppState {
        controller_version: StdRwLock::new(crate::ControllerVersionState::initial()),
        controller_cli_environment,
        log_level: RwLock::new(crate::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(config.credentials()),
        controller_web_auth_username: StdRwLock::new(config.controller_web_auth_username.clone()),
        controller_web_auth_password: StdRwLock::new(config.controller_web_auth_password.clone()),
        controller_web_jwt_key_current: StdRwLock::new(config.controller_web_jwt_key.clone()),
        session: RwLock::new(crate::SessionSnapshot::disconnected(&config)),
        server_address: StdRwLock::new(config.server_address.clone()),
        connected_server_address: StdRwLock::new(None),
        listeners: RwLock::new(crate::ListenerSnapshot::new(&config)),
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
        searches: RwLock::new(crate::SearchStore::new()),
        users: RwLock::new(crate::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(crate::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability key"),
        content_discovery: RwLock::new(crate::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(crate::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(crate::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(crate::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(crate::MessageStore::new()),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(crate::ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_profile,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            crate::compile_controller_regexes(
                &config.controller_search_request_filters,
                config.controller_case_sensitive_regex,
                config.controller_profile,
            )
            .expect("focused search filters"),
        ),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(crate::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(crate::LidarrSyncRuntimeState::new(
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
        failed_upload_peer_cooldowns: RwLock::new(crate::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            crate::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(crate::RoomStore::new()),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(crate::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(crate::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(crate::pod_channels::PodChannelStore::empty(
            &config.state_dir,
        )),
        pods: RwLock::new(crate::pods::PodStore::empty(&config.state_dir)),
        port_forwarding: crate::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(crate::TransferQueue::new(&config)),
        events: RwLock::new(crate::EventStore::new(crate::EVENT_HISTORY_LIMIT)),
        event_tx,
        webhooks: Arc::new(RwLock::new(crate::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(crate::Semaphore::new(crate::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(crate::Semaphore::new(crate::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(crate::Semaphore::new(crate::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(crate::Semaphore::new(
            config.core_workflow.incoming_search.concurrency,
        )),
        incoming_search_queue_depth: crate::AtomicUsize::new(0),
        download_requests: Arc::new(crate::Semaphore::new(2)),
        download_batch_requests: Arc::new(crate::Semaphore::new(1)),
        websocket_connections: Arc::new(crate::Semaphore::new(crate::MAX_WEBSOCKET_CONNECTIONS)),
        ftp_uploads: crate::ftp::FtpUploadQueue::default(),
        relay_cleanup: crate::relay::ConnectionCleanup::default(),
        external_visualizer_processes: Arc::new(crate::Semaphore::new(
            crate::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        visualizer_children:
            crate::external_visualizer_processes::ExternalVisualizerProcesses::default(),
        songid_run_slots: Arc::new(crate::Semaphore::new(
            config.media_services.song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(crate::CollectionStore::new()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(crate::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(crate::ContactStore::new()),
        sharegroups: RwLock::new(crate::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(crate::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(crate::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(crate::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(crate::RelayState::new()),
        runtime: RwLock::new(crate::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(crate::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(crate::BackfillState::default()),
        backfill_connections: Arc::new(crate::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(crate::SecurityState::new()),
        share_grants: RwLock::new(crate::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(crate::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(crate::IncomingShareStore::default()),
        library: RwLock::new(crate::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(crate::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(crate::SourceDiscoveryState::default()),
        destinations: RwLock::new(crate::DestinationStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        db: db.clone(),
        config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: crate::ManagedTaskRegistry::default(),
        rate_limiter,
        soulseek_safety: crate::rate_limit::SoulseekSafetyLimiter::new(
            crate::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(crate::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(crate::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(crate::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(crate::multisource::SwarmStore::default())),
        controller_features: crate::ControllerFeatureStore::new(
            crate::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(crate::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(crate::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(crate::RevokedJwtStore::default()),
        login_attempts: RwLock::new(crate::LoginAttemptStore::default()),
        pod_signature_stats: crate::PodSignatureStats::default(),
        pod_verification_stats: crate::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: crate::PodCoreRuntimeStats::default(),
    });
    if tokio::runtime::Handle::try_current().is_ok() {
        state.spawn_managed_task(crate::run_distributed_persistence_worker(
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

pub(super) fn write_ledger(file_name: &str, ledger: &[serde_json::Value]) {
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

pub(super) fn write_file_lifecycle_ledger(file_name: &str, ledger: &[serde_json::Value]) {
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

pub(super) async fn seed_old_message_for_database_cleanup(
    state: &Arc<crate::AppState>,
    db: &crate::persistence::DatabaseManager,
) -> crate::MessageRecord {
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
    db.insert_message(&crate::message_store::persisted_message_record(&record))
        .await
        .expect("persist old message before database cleanup");
    record
}
