use super::*;

#[derive(Debug)]
pub(super) struct AppState {
    pub(super) config: AppConfig,
    pub(super) controller_version: std::sync::RwLock<ControllerVersionState>,
    pub(super) controller_cli_environment: BTreeMap<String, String>,
    pub(super) log_level: RwLock<logging::LogLevel>,
    pub(super) runtime_credentials: RwLock<Option<LoginCredentials>>,
    pub(super) configured_credentials: RwLock<Option<LoginCredentials>>,
    pub(super) controller_web_auth_username: std::sync::RwLock<String>,
    pub(super) controller_web_auth_password: std::sync::RwLock<String>,
    pub(super) controller_web_jwt_key_current: std::sync::RwLock<String>,
    pub(super) session: RwLock<SessionSnapshot>,
    pub(super) server_address: std::sync::RwLock<String>,
    pub(super) connected_server_address: std::sync::RwLock<Option<String>>,
    pub(super) listeners: RwLock<ListenerSnapshot>,
    pub(super) distributed_network: RwLock<DistributedRuntime>,
    pub(super) soulseek_distributed_settings: RwLock<crate::config::SoulseekDistributedSettings>,
    pub(super) shares: RwLock<ShareIndexSnapshot>,
    pub(super) share_settings: RwLock<crate::config::ShareSettings>,
    pub(super) core_workflow_settings: RwLock<crate::config::CoreWorkflowSettings>,
    pub(super) advanced_networking: RwLock<crate::config::AdvancedNetworkingSettings>,
    pub(super) media_services: RwLock<crate::config::MediaAdvancedServiceSettings>,
    pub(super) share_lifecycle: RwLock<ShareLifecycleState>,
    pub(super) downloads_dir: std::sync::RwLock<PathBuf>,
    pub(super) incomplete_dir: std::sync::RwLock<PathBuf>,
    pub(super) download_completed_path_template: std::sync::RwLock<String>,
    pub(super) remote_file_management: std::sync::RwLock<bool>,
    pub(super) remote_configuration: std::sync::RwLock<bool>,
    pub(super) controller_no_config_watch: std::sync::RwLock<bool>,
    pub(super) controller_case_sensitive_regex: std::sync::RwLock<bool>,
    pub(super) user_info_description: std::sync::RwLock<String>,
    pub(super) user_info_picture: std::sync::RwLock<Option<PathBuf>>,
    pub(super) controller_options_validation_error: std::sync::RwLock<Option<String>>,
    pub(super) regular_listener_commands: Option<mpsc::Sender<ListenerCommand>>,
    pub(super) obfuscated_listener_commands: Option<mpsc::Sender<ListenerCommand>>,
    pub(super) advertised_port: std::sync::RwLock<u32>,
    pub(super) obfuscated_advertised_port: std::sync::RwLock<Option<u32>>,
    pub(super) searches: RwLock<SearchStore>,
    pub(super) users: RwLock<UserStore>,
    pub(super) mesh: RwLock<MeshState>,
    pub(super) capability_signing_key: SigningKey,
    pub(super) content_discovery: RwLock<content_discovery::ContentDiscoveryStore>,
    pub(super) realm_subject_indexes: RwLock<realm_subject_index::Store>,
    pub(super) browse: RwLock<BrowseStore>,
    pub(super) remote_path_encodings: RwLock<RemotePathEncodingRegistry>,
    pub(super) messages: RwLock<MessageStore>,
    pub(super) managed_blacklist: RwLock<ManagedBlacklistRuntime>,
    pub(super) search_request_filters: RwLock<Vec<ControllerRegex>>,
    pub(super) integration_settings: RwLock<crate::config::IntegrationSettings>,
    pub(super) source_feed_import_history: RwLock<SourceFeedImportHistoryStore>,
    pub(super) lidarr_sync_state: RwLock<LidarrSyncRuntimeState>,
    pub(super) lidarr_recent_imports: RwLock<BTreeMap<String, u64>>,
    pub(super) lidarr_import_gate: Semaphore,
    pub(super) private_message_auto_response_settings:
        RwLock<crate::config::PrivateMessageAutoResponseSettings>,
    pub(super) transfer_auto_retry_settings: RwLock<crate::config::TransferAutoRetrySettings>,
    pub(super) transfer_upload_settings: RwLock<crate::config::TransferUploadSettings>,
    pub(super) transfer_download_settings: RwLock<crate::config::TransferDownloadSettings>,
    pub(super) transfer_groups_settings: RwLock<crate::config::TransferGroupsSettings>,
    pub(super) failed_upload_peer_cooldowns: RwLock<UploadPeerCooldowns>,
    pub(super) private_message_auto_responses: RwLock<PrivateMessageAutoResponseTracker>,
    pub(super) rooms: RwLock<RoomStore>,
    pub(super) pod_join_replays: RwLock<PodJoinReplayStore>,
    pub(super) pod_membership_workflow: RwLock<PodMembershipWorkflowStore>,
    pub(super) pod_channels: RwLock<pod_channels::PodChannelStore>,
    pub(super) pods: RwLock<pods::PodStore>,
    pub(super) port_forwarding: port_forwarding::Manager,
    pub(super) private_gateway: Option<Arc<private_gateway::Gateway>>,
    pub(super) dht: Option<Arc<dht::Rendezvous>>,
    pub(super) transfers: RwLock<TransferQueue>,
    pub(super) events: RwLock<EventStore>,
    pub(super) event_tx: broadcast::Sender<EventRecord>,
    pub(super) webhooks: Arc<RwLock<webhooks::WebhookManager>>,
    pub(super) webhook_deliveries: Arc<Semaphore>,
    pub(super) share_scans: Arc<Semaphore>,
    pub(super) incoming_connections: Arc<Semaphore>,
    pub(super) incoming_connection_ips: std::sync::Mutex<BTreeMap<IpAddr, usize>>,
    pub(super) incoming_searches: Arc<Semaphore>,
    pub(super) incoming_search_queue_depth: AtomicUsize,
    pub(super) download_requests: Arc<Semaphore>,
    pub(super) download_batch_requests: Arc<Semaphore>,
    pub(super) websocket_connections: Arc<Semaphore>,
    pub(super) ftp_uploads: ftp::FtpUploadQueue,
    pub(super) external_visualizer_processes: Arc<Semaphore>,
    pub(super) songid_run_slots: Arc<Semaphore>,
    pub(super) songid_jobs: Option<mpsc::Sender<SongIdJob>>,
    pub(super) collections: RwLock<CollectionStore>,
    pub(super) wishlist: RwLock<WishlistStore>,
    pub(super) contacts: RwLock<ContactStore>,
    pub(super) sharegroups: RwLock<ShareGroupStore>,
    pub(super) user_notes: RwLock<UserNoteStore>,
    pub(super) interests: RwLock<InterestStore>,
    pub(super) now_playing: RwLock<NowPlayingStore>,
    pub(super) relay: RwLock<RelayState>,
    pub(super) runtime: RwLock<RuntimeCompatState>,
    pub(super) options_overlay: RwLock<ControllerOptionsOverlayState>,
    pub(super) diagnostics_allow_memory_dump: RwLock<bool>,
    pub(super) diagnostics_allow_remote_dump: RwLock<bool>,
    pub(super) backfill: RwLock<BackfillState>,
    pub(super) backfill_connections: Arc<Semaphore>,
    pub(super) pending_backfill_transfers: RwLock<BTreeMap<u32, PendingBackfillTransfer>>,
    pub(super) security: RwLock<SecurityState>,
    pub(super) share_grants: RwLock<ShareGrantStore>,
    pub(super) share_access_tokens: RwLock<ShareAccessTokenStore>,
    pub(super) incoming_shares: RwLock<IncomingShareStore>,
    pub(super) library: RwLock<LibraryStore>,
    pub(super) virtual_soulfind_v2: Arc<RwLock<virtual_soulfind_v2::State>>,
    pub(super) source_discovery: RwLock<SourceDiscoveryState>,
    pub(super) destinations: RwLock<DestinationStore>,
    pub(super) db: Option<crate::persistence::DatabaseManager>,
    pub(super) session_commands: mpsc::Sender<SessionCommand>,
    pub(super) pending_user_interests: RwLock<BTreeMap<String, Vec<UserInterestWaiter>>>,
    pub(super) lifecycle_commands: Option<mpsc::Sender<LifecycleCommand>>,
    pub(super) rate_limiter: rate_limit::RateLimiter,
    pub(super) soulseek_safety: rate_limit::SoulseekSafetyLimiter,
    pub(super) oauth_states: RwLock<OAuthStateStore>,
    pub(super) spotify_connection: RwLock<SpotifyConnectionStore>,
    pub(super) spotify_token_gate: Semaphore,
    pub(super) stream_tickets: RwLock<PreviewStreamTicketStore>,
    pub(super) multisource: Arc<RwLock<multisource::SwarmStore>>,
    pub(super) controller_features: ControllerFeatureStore,
    pub(super) peer_endpoints: RwLock<BTreeMap<String, (PeerAddress, u64)>>,
    pub(super) preview_streams: Arc<Semaphore>,
    pub(super) listening_party_stream_limits: RwLock<ListeningPartyStreamLimits>,
    pub(super) revoked_jwts: RwLock<RevokedJwtStore>,
    pub(super) login_attempts: RwLock<LoginAttemptStore>,
    pub(super) pod_signature_stats: PodSignatureStats,
    pub(super) pod_verification_stats: PodVerificationStats,
    /// Real, monotonic count of successful DHT publish/update calls --
    /// distinct from the number of currently-tracked publications (a
    /// republish overwrites the same `pod/dht/{id}` record), matching the
    /// oracle's `PodDhtPublisher`'s own all-time `_totalPublished` counter.
    pub(super) pod_dht_publish_count: std::sync::atomic::AtomicU64,
    pub(super) pod_dht_failed_publish_count: std::sync::atomic::AtomicU64,
    pub(super) pod_dht_publish_time_ms: std::sync::atomic::AtomicU64,
    pub(super) podcore_runtime_stats: PodCoreRuntimeStats,
    pub(super) distributed_persistence_snapshots: watch::Sender<DistributedPersistenceSnapshot>,
    pub(super) distributed_persistence_status: watch::Receiver<DistributedPersistenceStatus>,
    pub(super) share_index_persistence_lock: AsyncMutex<()>,
    pub(super) share_settings_generation: std::sync::atomic::AtomicU64,
    pub(super) user_persistence_lock: AsyncMutex<()>,
    pub(super) event_persistence_lock: AsyncMutex<()>,
    pub(super) browse_persistence_lock: AsyncMutex<()>,
    pub(super) message_persistence_lock: AsyncMutex<()>,
    pub(super) source_feed_import_history_persistence_lock: AsyncMutex<()>,
    pub(super) room_persistence_lock: AsyncMutex<()>,
    pub(super) share_scan_cancellation: Arc<Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>>,
    pub(super) collection_grant_persistence_lock: AsyncMutex<()>,
    pub(super) share_group_persistence_lock: AsyncMutex<()>,
    pub(super) contact_persistence_lock: AsyncMutex<()>,
    pub(super) wishlist_search_persistence_lock: AsyncMutex<()>,
    pub(super) search_persistence_lock: AsyncMutex<()>,
    pub(super) user_note_persistence_lock: AsyncMutex<()>,
    pub(super) interest_persistence_lock: AsyncMutex<()>,
    pub(super) now_playing_persistence_lock: AsyncMutex<()>,
    pub(super) webhook_persistence_lock: Arc<AsyncMutex<()>>,
    pub(super) runtime_persistence_lock: AsyncMutex<()>,
    pub(super) security_ban_persistence_lock: AsyncMutex<()>,
    pub(super) library_persistence_lock: AsyncMutex<()>,
    pub(super) managed_background_tasks: ManagedTaskRegistry,
    pub(super) oauth_persistence_lock: AsyncMutex<()>,
    pub(super) spotify_connection_persistence_lock: AsyncMutex<()>,
    pub(super) spotify_connection_generation: std::sync::atomic::AtomicU64,
}

pub(super) fn effective_downloads_dir(state: &AppState) -> PathBuf {
    state
        .downloads_dir
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(super) fn effective_incomplete_dir(state: &AppState) -> PathBuf {
    state
        .incomplete_dir
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(super) fn effective_download_completed_path_template(state: &AppState) -> String {
    state
        .download_completed_path_template
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(super) fn effective_server_address(state: &AppState) -> String {
    state
        .server_address
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(super) fn connected_server_address(state: &AppState) -> Option<String> {
    state
        .connected_server_address
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(super) fn clear_connected_server_address(state: &AppState) {
    *state
        .connected_server_address
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

pub(super) fn effective_remote_file_management(state: &AppState) -> bool {
    *state
        .remote_file_management
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn effective_remote_configuration(state: &AppState) -> bool {
    *state
        .remote_configuration
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn effective_controller_no_config_watch(state: &AppState) -> bool {
    *state
        .controller_no_config_watch
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn effective_advertised_port(state: &AppState) -> u32 {
    *state
        .advertised_port
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn effective_obfuscated_advertised_port(state: &AppState) -> Option<u32> {
    *state
        .obfuscated_advertised_port
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) async fn reconfigure_regular_listener(
    state: &AppState,
    bind: Option<String>,
) -> Result<bool, String> {
    let Some(commands) = state.regular_listener_commands.as_ref() else {
        return Ok(bind != state.config.listener_bind);
    };
    let (response, receiver) = oneshot::channel();
    commands
        .send(ListenerCommand::Reconfigure { bind, response })
        .await
        .map_err(|_| "regular listener manager is unavailable".to_owned())?;
    receiver
        .await
        .map_err(|_| "regular listener manager dropped its response".to_owned())?
}

pub(super) async fn reconfigure_obfuscated_listener(
    state: &AppState,
    bind: Option<String>,
) -> Result<bool, String> {
    let Some(commands) = state.obfuscated_listener_commands.as_ref() else {
        return Ok(bind != state.config.obfuscated_listener_bind);
    };
    let (response, receiver) = oneshot::channel();
    commands
        .send(ListenerCommand::Reconfigure { bind, response })
        .await
        .map_err(|_| "obfuscated listener manager is unavailable".to_owned())?;
    receiver
        .await
        .map_err(|_| "obfuscated listener manager dropped its response".to_owned())?
}

pub(super) async fn effective_user_info_description(state: &AppState) -> String {
    let base = state
        .user_info_description
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if state.config.controller_profile != ControllerProfile::Native {
        return base;
    }
    let now_playing = state.now_playing.read().await;
    let Some(record) = now_playing
        .records
        .iter()
        .max_by_key(|record| record.updated_at)
    else {
        return base;
    };
    format!(
        "{base}\n\n🎵 Listening to: {} – {}",
        record.artist, record.title
    )
}

pub(super) async fn effective_user_info_picture(state: &AppState) -> Option<Vec<u8>> {
    let path = state
        .user_info_picture
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()?;
    match fs::read(&path) {
        Ok(picture) => Some(picture),
        Err(error) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "soulseek",
                format!(
                    "Failed to read Soulseek picture {}: {error}",
                    path.display()
                ),
            )
            .await;
            None
        }
    }
}

impl AppState {
    pub(super) fn spawn_managed_task<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.managed_background_tasks.spawn(future);
    }

    pub(super) fn spawn_bounded_http_task<F>(
        self: &Arc<Self>,
        connections: &Arc<Semaphore>,
        future: F,
    ) -> bool
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.managed_background_tasks
            .spawn_bounded_http(connections, future)
    }

    pub(super) async fn shutdown_managed_tasks(&self) {
        self.ftp_uploads.close();
        self.managed_background_tasks.shutdown().await;
        self.port_forwarding.shutdown().await;
        if let Some(gateway) = self.private_gateway.as_ref() {
            gateway.clear_runtime_connections().await;
        }
        {
            let mut runtime = self.runtime.write().await;
            runtime.bridge_active_clients.clear();
            runtime.set_bridge_running(
                false,
                self.config.media_services.virtual_soulfind.bridge.enabled,
            );
        }
        self.multisource.write().await.fail_unfinished(
            "daemon shut down before the swarm completed",
            unix_timestamp(),
        );
        if let Err(error) = self.persist_distributed_shutdown_snapshot().await {
            eprintln!("distributed persistence shutdown flush failed: {error}");
        }
    }

    pub(super) async fn persist_distributed_shutdown_snapshot(&self) -> Result<(), String> {
        let Some(db) = self.db.as_ref() else {
            return Ok(());
        };
        let snapshot = self.distributed_network.read().await.persistence_snapshot();
        if snapshot.revision == 0 {
            return Ok(());
        }
        match time::timeout(MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, snapshot.save(db)).await {
            Ok(result) => result,
            Err(_) => Err("timed out while saving the final distributed snapshot".to_owned()),
        }
    }
}
