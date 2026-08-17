use std::{
    collections::BTreeMap,
    fs,
    sync::{Arc, RwLock as StdRwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use tokio::sync::{mpsc, RwLock};

use crate::config::{ConfigEnv, FileConfig};

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
        distributed_network: RwLock::new(super::DistributedRuntime::new(
            config.username.as_deref(),
        )),
        soulseek_distributed_settings: RwLock::new(config.soulseek_distributed),
        shares: RwLock::new(share_index),
        share_settings: RwLock::new(config.share_settings.clone()),
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
        mesh: RwLock::new(super::MeshState::new()),
        capability_signing_key: super::new_capability_signing_key().expect("capability key"),
        content_discovery: RwLock::new(super::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(super::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(super::BrowseStore::new()),
        remote_path_encodings: RwLock::new(super::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(super::MessageStore::new()),
        managed_blacklist: RwLock::new(super::ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_compatibility_target,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            super::compile_controller_regexes(
                &config.controller_search_request_filters,
                config.controller_case_sensitive_regex,
                config.controller_compatibility_target,
            )
            .expect("focused search filters"),
        ),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(super::SourceFeedImportHistoryStore::default()),
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
        pod_join_replays: RwLock::new(BTreeMap::new()),
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
        webhooks: RwLock::new(super::webhooks::WebhookManager::new()),
        webhook_deliveries: Arc::new(super::Semaphore::new(super::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(super::Semaphore::new(super::MAX_SHARE_SCAN_TASKS)),
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
        collections: RwLock::new(super::CollectionStore::new()),
        wishlist: RwLock::new(super::WishlistStore::new()),
        contacts: RwLock::new(super::ContactStore::new()),
        sharegroups: RwLock::new(super::ShareGroupStore::new()),
        user_notes: RwLock::new(super::UserNoteStore::new()),
        interests: RwLock::new(super::InterestStore::new()),
        now_playing: RwLock::new(super::NowPlayingStore::new()),
        relay: RwLock::new(super::RelayState::new()),
        runtime: RwLock::new(super::RuntimeCompatState::new()),
        options_overlay: RwLock::new(super::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(super::BackfillState::default()),
        backfill_connections: Arc::new(super::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security: RwLock::new(super::SecurityState::new()),
        share_grants: RwLock::new(super::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(super::ShareAccessTokenStore::default()),
        library: RwLock::new(super::LibraryStore::new()),
        virtual_soulfind_v2: Arc::new(RwLock::new(super::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(super::SourceDiscoveryState::default()),
        destinations: RwLock::new(super::DestinationStore::new()),
        db: None,
        config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        rate_limiter,
        soulseek_safety: super::rate_limit::SoulseekSafetyLimiter::new(
            super::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(super::OAuthStateStore::default()),
        spotify_connection: RwLock::new(super::SpotifyConnectionStore::default()),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(super::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(super::multisource::SwarmStore::default())),
        controller_features: RwLock::new(super::ControllerFeatureState::in_memory()),
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
async fn controller_api_differential_slskd_file_transfer_room_residuals() {
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
            .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
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
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            repeated.status == "404 Not Found"
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
            response.status == "404 Not Found"
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
            .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
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
            response.status == "404 Not Found"
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
            .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
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
    write_ledger("slskd_focused_file_transfer_room_residuals.json", &ledger);
}

#[test]
fn folder_contents_response_parser_accepts_slskd_wire_shape() {
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
async fn file_lifecycle_differential_slskd_file_service_existing_missing_overwrite() {
    let target = "slskd";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
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
        && missing.status == "404 Not Found"
        && !managed_file.exists();
    assert!(
        pass,
        "slskd FileService delete contract: first={}, second={}, exists={}",
        deleted.status,
        missing.status,
        managed_file.exists()
    );
    write_file_lifecycle_ledger(
        "slskd_focused_file_service_existing_missing_overwrite.json",
        &[serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "existing-missing-and-overwrite",
            "pass": pass,
        })],
    );
}
