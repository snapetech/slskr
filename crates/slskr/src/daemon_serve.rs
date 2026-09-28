use super::*;

/// Registers a background task that waits for a shutdown signal (SIGTERM,
/// SIGINT, and SIGQUIT on Unix; Ctrl-C on other platforms) and runs the same
/// graceful shutdown sequence as `DELETE /api/application`, matching the
/// oracle's `PosixSignalRegistration`-based teardown -- without this, a
/// `docker stop` or `systemctl stop` hard-kills the process with no drain
/// or clean disconnect at all.
fn spawn_signal_shutdown_handler(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        wait_for_shutdown_signal().await;
        record_daemon_log(
            &task_state,
            logging::LogLevel::Info,
            "lifecycle",
            "shutdown signal received; disconnecting and stopping".to_owned(),
        )
        .await;
        initiate_graceful_shutdown(&task_state).await;
    });
}

#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    async fn wait_or_pending(kind: SignalKind) {
        match signal(kind) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    }
    tokio::select! {
        () = wait_or_pending(SignalKind::terminate()) => {}
        () = wait_or_pending(SignalKind::interrupt()) => {}
        () = wait_or_pending(SignalKind::quit()) => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn spawn_replacement_process() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    std::process::Command::new(executable)
        .args(std::env::args_os().skip(1))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to restart slskr: {error}"))
}

pub(super) fn load_pod_stores(
    state_dir: &Path,
    gold_star_club_enabled: bool,
) -> Result<(pod_channels::PodChannelStore, pods::PodStore), String> {
    let mut channels = pod_channels::PodChannelStore::load(state_dir)?;
    let pod_store = pods::PodStore::load_with_setting(state_dir, gold_star_club_enabled)?;
    let removed = channels.remove_orphaned_channels(&pod_store)?;
    if removed > 0 {
        eprintln!("removed {removed} orphaned pod-channel messages during startup recovery");
    }
    Ok((channels, pod_store))
}

pub(super) async fn serve(invocation: ServeInvocation) -> Result<(), String> {
    let (config_file, file_config) = config::load_file_config()?;
    let config = AppConfig::from_layers(
        config_file,
        file_config,
        &ControllerCliEnv {
            values: &invocation.config_environment,
        },
    )?;
    let once = invocation.once;
    if !config.controller_no_logo {
        print_controller_logo(config.controller_profile);
    }
    if let Some(error) = frozen_obfuscation_startup_error(&config) {
        // Frozen native profile accepts this combination during options validation,
        // fails while constructing the runtime, logs a fatal error, and exits 0.
        eprintln!("{error}");
        return Ok(());
    }
    ensure_private_state_dir(&config.state_dir)?;
    ensure_private_storage_dir(&config.downloads_dir, "downloads")?;
    ensure_private_storage_dir(&config.incomplete_dir, "incomplete")?;
    initialize_telemetry(&config).await?;
    let address = config.http_bind;
    let (session_commands, session_receiver) =
        mpsc::channel(config.soulseek_connection.buffer_write_queue);
    let (regular_listener_commands, regular_listener_receiver) = mpsc::channel(8);
    let (obfuscated_listener_commands, obfuscated_listener_receiver) = mpsc::channel(8);
    let (lifecycle_commands, mut lifecycle_receiver) = mpsc::channel(2);
    let lifecycle_commands = std::env::var_os("SLSKR_CONTROLLER_AUDIT_MODE")
        .is_none()
        .then_some(lifecycle_commands);
    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
    let (songid_job_sender, songid_job_receiver) = mpsc::channel(4096);
    let db = if config.daemon_flags.volatile {
        Some(
            crate::persistence::DatabaseManager::in_memory()
                .await
                .map_err(|error| {
                    format!("failed to open volatile persistence database: {error}")
                })?,
        )
    } else if config.persistence_enabled || config.daemon_flags.force_migrations {
        let db_path = config.state_dir.join("slskr.db");
        Some(
            crate::persistence::DatabaseManager::new(db_path.to_str().unwrap_or("slskr.db"))
                .await
                .map_err(|error| format!("failed to open persistence database: {error}"))?,
        )
    } else {
        None
    };
    if config.controller_no_start {
        println!("Quitting because 'no-start' option is enabled");
        return Ok(());
    }
    config.validate_controller_startup_hardening()?;
    if let Some(database) = db.as_ref() {
        database
            .fail_unconfirmed_webhook_logs(
                "delivery outcome unknown: daemon restarted before confirmation",
            )
            .await
            .map_err(|error| {
                format!("failed to reconcile interrupted webhook deliveries: {error}")
            })?;
    }
    let share_index = if config.controller_no_share_scan {
        ShareIndexSnapshot::uninitialized(&config)
    } else if let Some(db) = db.as_ref() {
        let records = db
            .list_share_files(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted share index: {error}"))?;
        let configured_aliases = config
            .share_settings
            .directories
            .iter()
            .filter(|directory| !directory.is_excluded)
            .map(|directory| directory.alias.as_str())
            .collect::<HashSet<_>>();
        let persisted_aliases = records
            .iter()
            .map(|record| record.root_label.as_str())
            .collect::<HashSet<_>>();
        let newest_cache_record = records
            .iter()
            .filter_map(|record| u64::try_from(record.updated_at).ok())
            .max();
        let cache_is_fresh = config
            .share_settings
            .cache_retention
            .is_none_or(|retention| {
                newest_cache_record.is_some_and(|updated_at| {
                    unix_timestamp().saturating_sub(updated_at) <= retention.as_secs()
                })
            });
        if config.controller_force_share_scan
            || records.is_empty()
            || configured_aliases != persisted_aliases
            || !cache_is_fresh
        {
            let scanned_share_index = build_share_index(&config);
            let records = persisted_share_file_records(&scanned_share_index);
            db.replace_share_files(&records)
                .await
                .map_err(|error| format!("failed to persist share index: {error}"))?;
            scanned_share_index
        } else {
            ShareIndexSnapshot::from_persisted(&config, records)
        }
    } else {
        build_share_index(&config)
    };
    let search_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_searches(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted searches: {error}"))?;
        let result_records = db
            .list_search_results(None, EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted search results: {error}"))?;
        let identities = db
            .list_search_identities()
            .await
            .map_err(|error| format!("failed to load persisted search identities: {error}"))?;
        SearchStore::from_persisted_with_results_and_identities(records, result_records, identities)
    } else {
        SearchStore::new()
    };
    let message_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_messages(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted messages: {error}"))?;
        MessageStore::from_persisted(records)
    } else {
        MessageStore::new()
    };
    let user_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_user_projections(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted users: {error}"))?;
        UserStore::from_persisted(records)
    } else {
        UserStore::new()
    };
    let mut room_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_subscribed_rooms()
            .await
            .map_err(|error| format!("failed to load persisted rooms: {error}"))?;
        RoomStore::from_persisted(records)
    } else {
        RoomStore::new()
    };
    room_store.merge_configured(&config.core_workflow.rooms);
    // Browse state is transient - don't rehydrate persisted remote browse results
    // Frozen targets perform fresh browse operations and don't persist remote results
    let browse_store = BrowseStore::new();
    let user_note_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_user_notes(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted user notes: {error}"))?;
        UserNoteStore::from_persisted(records)
    } else {
        UserNoteStore::new()
    };
    let mut interest_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_interests(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted interests: {error}"))?;
        InterestStore::from_persisted(records)
    } else {
        InterestStore::new()
    };
    interest_store.merge_configured(
        &config.core_workflow.liked_interests,
        &config.core_workflow.hated_interests,
    );
    if config.advanced_networking.mesh.enabled
        && config.advanced_networking.mesh.enable_soulseek_rendezvous
    {
        let _ = interest_store.add_liked(MESH_RENDEZVOUS_INTEREST_TAG.to_owned());
    }
    let security_state = if let Some(db) = db.as_ref() {
        let records = db
            .list_security_bans()
            .await
            .map_err(|error| format!("failed to load persisted security bans: {error}"))?;
        SecurityState::from_persisted(records)
    } else {
        SecurityState::new()
    };
    let wishlist_store = load_wishlist_store(db.as_ref()).await?;
    let contact_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_contacts(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted contacts: {error}"))?;
        ContactStore::from_persisted(records)
    } else {
        ContactStore::new()
    };
    let mut share_grant_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_share_grants(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted share grants: {error}"))?;
        ShareGrantStore::from_persisted(records)
    } else {
        ShareGrantStore::new()
    };
    let share_group_store = if let Some(db) = db.as_ref() {
        let groups = db
            .list_share_groups(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted share groups: {error}"))?;
        let members = db
            .list_share_group_members(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted share group members: {error}"))?;
        ShareGroupStore::from_persisted(groups, members)
    } else {
        ShareGroupStore::new()
    };
    let collection_store = if let Some(db) = db.as_ref() {
        let collections = db
            .list_collections(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted collections: {error}"))?;
        let items = db
            .list_collection_items(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted collection items: {error}"))?;
        CollectionStore::from_persisted(collections, items)
    } else {
        CollectionStore::new()
    };
    let collection_ids = collection_store
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<HashSet<_>>();
    let stale_grant_ids = share_grant_store
        .records
        .iter()
        .filter(|grant| !collection_ids.contains(grant.collection_id.as_str()))
        .map(|grant| grant.id.clone())
        .collect::<Vec<_>>();
    share_grant_store
        .records
        .retain(|grant| collection_ids.contains(grant.collection_id.as_str()));
    if let Some(db) = db.as_ref() {
        for id in stale_grant_ids {
            db.delete_share_grant(&id)
                .await
                .map_err(|error| format!("failed to delete stale share grant: {error}"))?;
        }
    }
    let valid_grant_ids = share_grant_store
        .records
        .iter()
        .map(|grant| grant.id.as_str())
        .collect::<HashSet<_>>();
    let share_access_token_store = if let Some(db) = db.as_ref() {
        let now = i64::try_from(unix_timestamp()).unwrap_or(i64::MAX);
        db.delete_expired_share_access_tokens(now)
            .await
            .map_err(|error| {
                format!("failed to delete expired persisted share access tokens: {error}")
            })?;
        let records = db
            .list_share_access_tokens(now, MAX_SHARE_ACCESS_TOKENS as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted share access tokens: {error}"))?;
        ShareAccessTokenStore::from_persisted(records, &valid_grant_ids)
    } else {
        ShareAccessTokenStore::default()
    };
    drop(valid_grant_ids);
    let library_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_library_items(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted library items: {error}"))?;
        LibraryStore::from_persisted(records)
    } else {
        LibraryStore::new()
    };
    let destination_store = if !config.core_workflow.destinations.is_empty() {
        DestinationStore::from_config(&config.downloads_dir, &config.core_workflow.destinations)
    } else if let Some(db) = db.as_ref() {
        let records = db
            .list_destinations(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted destinations: {error}"))?;
        if records.is_empty() {
            DestinationStore::from_config(&config.downloads_dir, &[])
        } else {
            DestinationStore::from_persisted(records)
        }
    } else {
        DestinationStore::from_config(&config.downloads_dir, &[])
    };
    let now_playing_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_now_playing(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted now-playing records: {error}"))?;
        NowPlayingStore::from_persisted(records)
    } else {
        NowPlayingStore::new()
    };
    let event_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_events(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted events: {error}"))?;
        EventStore::from_persisted(records, EVENT_HISTORY_LIMIT)
    } else {
        EventStore::new(EVENT_HISTORY_LIMIT)
    };
    let runtime_compat_record = if let Some(db) = db.as_ref() {
        db.get_runtime_compat_state().await.map_err(|error| {
            format!("failed to load persisted runtime compatibility state: {error}")
        })?
    } else {
        None
    };
    let oauth_state_store = load_oauth_state_store(db.as_ref()).await?;
    let webhook_manager = if let Some(db) = db.as_ref() {
        let records = db
            .list_webhooks()
            .await
            .map_err(|error| format!("failed to load persisted webhooks: {error}"))?;
        webhooks::WebhookManager::from_webhooks(
            records
                .into_iter()
                .filter_map(webhook_from_persisted)
                .collect(),
        )
    } else {
        webhooks::WebhookManager::new()
    };
    let mut relay_state = runtime_compat_record
        .as_ref()
        .map(RelayState::from_persisted)
        .unwrap_or_else(RelayState::new);
    relay_state
        .protocol
        .restore_persisted_share_uploads(
            &config.state_dir.join("relay").join("incoming"),
            config.controller_profile,
        )
        .await?;
    let mut runtime_compat_state = runtime_compat_record
        .as_ref()
        .map(RuntimeCompatState::try_from_persisted)
        .transpose()
        .map_err(|error| format!("failed to load persisted runtime compatibility state: {error}"))?
        .unwrap_or_else(RuntimeCompatState::new);
    if config.advanced_networking.relay.enabled {
        relay_state.set_enabled(true);
        runtime_compat_state.set_relay_agent(config.advanced_networking.relay.mode == "agent");
    }

    let rate_limiter = rate_limit::RateLimiter::new(rate_limit::RateLimitConfig {
        max_requests_anonymous: config.api_rate_limit_anonymous,
        max_requests_authenticated: config.api_rate_limit_authenticated,
        window_seconds: 60,
        enabled: true,
    });

    let mut content_discovery_store =
        content_discovery::ContentDiscoveryStore::load(&config.state_dir)?;
    if let Some(db) = db.as_ref() {
        let persisted = db
            .list_hash_db_entries()
            .await
            .map_err(|error| format!("failed to load persisted HashDb entries: {error}"))?;
        if persisted.is_empty() {
            if !content_discovery_store.hash_entries().is_empty() {
                let records = content_discovery_store
                    .hash_entries()
                    .iter()
                    .map(persisted_hash_db_record)
                    .collect::<Vec<_>>();
                db.replace_hash_db_snapshot(
                    &records,
                    i64::try_from(content_discovery_store.latest_seq()).unwrap_or(i64::MAX),
                )
                .await
                .map_err(|error| format!("failed to migrate persisted HashDb entries: {error}"))?;
            }
        } else {
            let entries = persisted
                .into_iter()
                .map(hash_db_entry_from_persistence)
                .collect::<Result<Vec<_>, _>>()?;
            let latest_seq = db
                .get_hash_db_state("latest_seq")
                .await
                .map_err(|error| format!("failed to load persisted HashDb state: {error}"))?
                .and_then(|record| record.value)
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or_else(|| entries.iter().map(|entry| entry.seq_id).max().unwrap_or(0));
            content_discovery_store.restore_hash_entries(entries, latest_seq)?;
        }
    }
    let realm_subject_index_store = realm_subject_index::Store::load_with_identity(
        &config.state_dir,
        &config.realm.id,
        config.realm.governance_roots.iter(),
    )?;
    let controller_feature_state = ControllerFeatureState::load(&config.state_dir)?;
    let mut controller_options_state = ControllerOptionsOverlayState::load(&config)?;
    controller_options_state.command_line_environment = invocation.config_environment.clone();
    let backfill_state = BackfillState::load(&config.state_dir)?;
    let (pod_channel_store, pod_store) = load_pod_stores(
        &config.state_dir,
        config.advanced_networking.gold_star_club_autojoin,
    )?;
    let shared_mesh_tcp = config.shared_mesh_tcp();
    let shared_dht_udp_bind = config.overlay_bind.and_then(|bind| {
        let dht = &config.advanced_networking.dht;
        let overlay = &config.advanced_networking.overlay;
        let mesh = &config.advanced_networking.mesh;
        let shares_public_udp = config.dht_enabled
            && mesh.enabled
            && mesh.enable_dht
            && mesh.enable_overlay
            && overlay.enable
            && (overlay.listen_port == dht.dht_port
                || (overlay.enable_quic
                    && overlay.share_quic_with_dht_port
                    && overlay.quic_listen_port == dht.dht_port)
                || (config.advanced_networking.overlay_data.enable
                    && config.advanced_networking.overlay_data.share_with_dht_port
                    && config.advanced_networking.overlay_data.listen_port == dht.dht_port));
        shares_public_udp.then_some(SocketAddr::new(bind.ip(), dht.dht_port))
    });
    let native_public_udp_socket = if shared_mesh_tcp {
        let bind = config.overlay_bind.expect("shared peer bind was validated");
        let socket = std::net::UdpSocket::bind(bind)
            .map_err(|error| format!("shared peer UDP bind failed: {error}"))?;
        socket
            .set_nonblocking(true)
            .map_err(|error| format!("shared peer UDP setup failed: {error}"))?;
        Some(Arc::new(socket))
    } else {
        None
    };
    let dht = if config.dht_enabled && config.advanced_networking.mesh.enable_dht {
        Some(Arc::new(dht::Rendezvous::new_with_udp_socket(
            &config.advanced_networking.dht,
            shared_mesh_tcp || shared_dht_udp_bind.is_some(),
            native_public_udp_socket.clone(),
        )?))
    } else {
        None
    };
    let private_gateway = if config.advanced_networking.mesh.enabled
        && config.advanced_networking.mesh.enable_overlay
    {
        if let Some(bind) = config.overlay_bind {
            let overlay = &config.advanced_networking.overlay;
            let quic_shared_with_dht = overlay.enable_quic
                && overlay.share_quic_with_dht_port
                && shared_dht_udp_bind.is_some()
                && overlay.quic_listen_port == config.advanced_networking.dht.dht_port;
            let quic_data_shared_with_dht = config.advanced_networking.overlay_data.enable
                && config.advanced_networking.overlay_data.share_with_dht_port
                && shared_dht_udp_bind.is_some()
                && config.advanced_networking.overlay_data.listen_port
                    == config.advanced_networking.dht.dht_port;
            let quic_bind = config.advanced_networking.overlay.enable_quic.then(|| {
                if shared_mesh_tcp {
                    return bind;
                }
                let address = if quic_shared_with_dht {
                    IpAddr::V4(Ipv4Addr::LOCALHOST)
                } else {
                    bind.ip()
                };
                let port = if quic_shared_with_dht {
                    overlay.quic_backend_listen_port
                } else {
                    overlay.quic_listen_port
                };
                SocketAddr::new(address, port)
            });
            let quic_data_bind = config.advanced_networking.overlay_data.enable.then(|| {
                if shared_mesh_tcp {
                    return bind;
                }
                let shared = quic_data_shared_with_dht;
                let address = if shared {
                    IpAddr::V4(Ipv4Addr::LOCALHOST)
                } else {
                    bind.ip()
                };
                let port = if shared {
                    config.advanced_networking.overlay_data.backend_listen_port
                } else {
                    config.advanced_networking.overlay_data.listen_port
                };
                SocketAddr::new(address, port)
            });
            let quic_proxy_bind = (quic_shared_with_dht || quic_data_shared_with_dht).then(|| {
                shared_dht_udp_bind.expect("QUIC shared mode requires a shared DHT UDP bind")
            });
            let quic_data_policy = config.advanced_networking.overlay_data.enable.then(|| {
                private_gateway::QuicDataPolicy {
                    relay_authentication_token: config
                        .advanced_networking
                        .overlay_data
                        .relay_authentication_token
                        .clone(),
                    allowed_relay_destinations: config
                        .advanced_networking
                        .overlay_data
                        .allowed_relay_destinations
                        .clone(),
                    max_concurrent_relays: config
                        .advanced_networking
                        .overlay_data
                        .max_concurrent_relays,
                    max_relay_bytes_per_direction: config
                        .advanced_networking
                        .overlay_data
                        .max_relay_bytes_per_direction,
                    max_relay_duration: config.advanced_networking.overlay_data.max_relay_duration,
                }
            });
            let gateway = if shared_mesh_tcp {
                private_gateway::Gateway::load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share_shared_tcp(
                    bind,
                    &config.state_dir,
                    quic_bind,
                    quic_data_bind,
                    quic_proxy_bind,
                    shared_dht_udp_bind,
                    native_public_udp_socket.clone().or_else(|| dht.as_ref()
                        .and_then(|rendezvous| rendezvous.shared_udp_socket())),
                    dht.as_ref()
                        .and_then(|rendezvous| rendezvous.shared_udp_backend()),
                    quic_data_policy,
                    config
                        .advanced_networking
                        .overlay_data
                        .max_concurrent_streams,
                    quic_data_shared_with_dht,
                )
                .await?
            } else {
                private_gateway::Gateway::load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share(
                    bind,
                    &config.state_dir,
                    quic_bind,
                    quic_data_bind,
                    quic_proxy_bind,
                    shared_dht_udp_bind,
                    dht.as_ref()
                        .and_then(|rendezvous| rendezvous.shared_udp_socket()),
                    dht.as_ref()
                        .and_then(|rendezvous| rendezvous.shared_udp_backend()),
                    quic_data_policy,
                    config
                        .advanced_networking
                        .overlay_data
                        .max_concurrent_streams,
                    quic_data_shared_with_dht,
                )
                .await?
            };
            Some(Arc::new(gateway))
        } else {
            None
        }
    } else {
        None
    };
    let capability_signing_key = load_or_create_capability_signing_key(&config.state_dir)?;
    let spotify_connection =
        load_spotify_connection_store(&config.state_dir, &capability_signing_key);
    let share_lifecycle = if config.controller_no_share_scan {
        ShareLifecycleState::uninitialized()
    } else {
        ShareLifecycleState::from_snapshot(&share_index)
    };
    let share_settings = config.share_settings.clone();
    let search_request_filters = compile_controller_regexes(
        &config.controller_search_request_filters,
        config.controller_case_sensitive_regex,
        config.controller_profile,
    )?;
    let controller_cli_environment = invocation.config_environment.clone();
    let revoked_jwts = RevokedJwtStore::load(&config.state_dir)?;
    let mut mesh_state = MeshState::from_settings(&config.advanced_networking.mesh);
    if let Some(records) = controller_feature_state
        .get("hashdb/peers")
        .and_then(|value| value.get("peers"))
        .and_then(serde_json::Value::as_array)
    {
        mesh_state.restore_persisted_capabilities(records);
    }
    let distributed_runtime = DistributedRuntime::new(config.username.as_deref());
    let (distributed_persistence_snapshots, distributed_persistence_receiver) =
        watch::channel(distributed_runtime.persistence_snapshot());
    let (distributed_persistence_status_sender, distributed_persistence_status) =
        watch::channel(DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });
    let state = Arc::new(AppState {
        controller_version: std::sync::RwLock::new(ControllerVersionState::initial()),
        controller_cli_environment,
        log_level: RwLock::new(if config.controller_debug {
            logging::LogLevel::Debug
        } else {
            logging::LogConfig::parse_level(&config.log_level).unwrap_or(logging::LogLevel::Info)
        }),
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
        session: RwLock::new(SessionSnapshot::disconnected(&config)),
        server_address: std::sync::RwLock::new(config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(ListenerSnapshot::new(&config)),
        distributed_network: RwLock::new(distributed_runtime),
        distributed_persistence_snapshots,
        distributed_persistence_status,
        soulseek_distributed_settings: RwLock::new(config.soulseek_distributed),
        shares: RwLock::new(share_index),
        share_settings: RwLock::new(share_settings),
        share_index_persistence_lock: AsyncMutex::new(()),
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
        regular_listener_commands: Some(regular_listener_commands),
        obfuscated_listener_commands: Some(obfuscated_listener_commands),
        advertised_port: std::sync::RwLock::new(config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(
            config
                .obfuscation_enabled
                .then_some(config.obfuscated_advertised_port)
                .flatten(),
        ),
        searches: RwLock::new(search_store),
        users: RwLock::new(user_store),
        user_persistence_lock: AsyncMutex::new(()),
        event_persistence_lock: AsyncMutex::new(()),
        mesh: RwLock::new(mesh_state),
        capability_signing_key,
        content_discovery: RwLock::new(content_discovery_store),
        realm_subject_indexes: RwLock::new(realm_subject_index_store),
        browse: RwLock::new(browse_store),
        browse_persistence_lock: AsyncMutex::new(()),
        remote_path_encodings: RwLock::new(RemotePathEncodingRegistry::default()),
        messages: RwLock::new(message_store),
        message_persistence_lock: AsyncMutex::new(()),
        managed_blacklist: RwLock::new(ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_profile,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(search_request_filters),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(SourceFeedImportHistoryStore::load(
            &config.state_dir,
        )),
        source_feed_import_history_persistence_lock: AsyncMutex::new(()),
        lidarr_sync_state: RwLock::new(LidarrSyncRuntimeState::new(&config.integrations.lidarr)),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(config.transfer_auto_retry.clone()),
        transfer_upload_settings: RwLock::new(config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(PrivateMessageAutoResponseTracker::default()),
        rooms: RwLock::new(room_store),
        room_persistence_lock: AsyncMutex::new(()),
        pod_join_replays: RwLock::new(PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(pod_channel_store),
        pods: RwLock::new(pod_store),
        port_forwarding: port_forwarding::Manager::new(),
        private_gateway,
        dht,
        transfers: RwLock::new(TransferQueue::new(&config)),
        events: RwLock::new(event_store),
        event_tx,
        webhooks: Arc::new(RwLock::new(webhook_manager)),
        webhook_deliveries: Arc::new(Semaphore::new(MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(Semaphore::new(MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(Mutex::new(None)),
        incoming_connections: Arc::new(Semaphore::new(10_000)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(Semaphore::new(
            config.core_workflow.incoming_search.concurrency,
        )),
        incoming_search_queue_depth: AtomicUsize::new(0),
        download_requests: Arc::new(Semaphore::new(2)),
        download_batch_requests: Arc::new(Semaphore::new(1)),
        websocket_connections: Arc::new(Semaphore::new(MAX_WEBSOCKET_CONNECTIONS)),
        ftp_uploads: crate::ftp::FtpUploadQueue::default(),
        relay_cleanup: crate::relay::ConnectionCleanup::default(),
        external_visualizer_processes: Arc::new(Semaphore::new(MAX_EXTERNAL_VISUALIZER_PROCESSES)),
        visualizer_children: external_visualizer_processes::ExternalVisualizerProcesses::default(),
        songid_run_slots: Arc::new(Semaphore::new(
            config.media_services.song_id_max_concurrent_runs,
        )),
        songid_jobs: Some(songid_job_sender),
        collections: RwLock::new(collection_store),
        collection_grant_persistence_lock: AsyncMutex::new(()),
        share_group_persistence_lock: AsyncMutex::new(()),
        contact_persistence_lock: AsyncMutex::new(()),
        wishlist_search_persistence_lock: AsyncMutex::new(()),
        search_persistence_lock: AsyncMutex::new(()),
        wishlist: RwLock::new(wishlist_store),
        contacts: RwLock::new(contact_store),
        sharegroups: RwLock::new(share_group_store),
        user_note_persistence_lock: AsyncMutex::new(()),
        user_notes: RwLock::new(user_note_store),
        interest_persistence_lock: AsyncMutex::new(()),
        interests: RwLock::new(interest_store),
        now_playing_persistence_lock: AsyncMutex::new(()),
        now_playing: RwLock::new(now_playing_store),
        webhook_persistence_lock: Arc::new(AsyncMutex::new(())),
        relay: RwLock::new(relay_state),
        runtime: RwLock::new(runtime_compat_state),
        runtime_persistence_lock: AsyncMutex::new(()),
        options_overlay: RwLock::new(controller_options_state),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(backfill_state),
        backfill_connections: Arc::new(Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: AsyncMutex::new(()),
        security: RwLock::new(security_state),
        share_grants: RwLock::new(share_grant_store),
        share_access_tokens: RwLock::new(share_access_token_store),
        incoming_shares: RwLock::new(IncomingShareStore::default()),
        library: RwLock::new(library_store),
        library_persistence_lock: AsyncMutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(SourceDiscoveryState::default()),
        destinations: RwLock::new(destination_store),
        db,
        config,
        session_commands,
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands,
        managed_background_tasks: ManagedTaskRegistry::default(),
        rate_limiter,
        soulseek_safety: rate_limit::SoulseekSafetyLimiter::new(
            rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(oauth_state_store),
        oauth_persistence_lock: AsyncMutex::new(()),
        spotify_connection: RwLock::new(spotify_connection),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(multisource::SwarmStore::default())),
        controller_features: ControllerFeatureStore::new(controller_feature_state),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(Semaphore::new(MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(revoked_jwts),
        login_attempts: RwLock::new(LoginAttemptStore::default()),
        pod_signature_stats: PodSignatureStats::default(),
        pod_verification_stats: PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: PodCoreRuntimeStats::default(),
    });
    let visualizer_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            visualizer_state.visualizer_children.reap_finished();
        }
    });
    state.spawn_managed_task(run_distributed_persistence_worker(
        state.db.clone(),
        distributed_persistence_receiver,
        distributed_persistence_status_sender,
    ));
    if gold_star_club_available(&state) {
        let result = state.pods.write().await.ensure_gold_star_club();
        if let Err(error) = result {
            record_daemon_log(
                &state,
                logging::LogLevel::Warn,
                "podcore",
                format!("Gold Star Club initialization failed: {error}"),
            )
            .await;
        }
    }
    if let Some(db) = state.db.as_ref() {
        {
            state
                .transfers
                .write()
                .await
                .rehydrate_from_database(db)
                .await;
        }
    }
    persist_transfer_durability(&state).await;
    spawn_songid_workers(Arc::clone(&state), songid_job_receiver);
    requeue_persisted_songid_runs(&state).await;
    start_controller_version_check(Arc::clone(&state)).await;
    match credential_store::load(&state.config) {
        Ok(Some(stored)) => {
            let username = redact_username(&stored.credentials.username);
            if stored.source != "config" {
                let mut runtime_credentials = state.runtime_credentials.write().await;
                *runtime_credentials = Some(stored.credentials);
            }
            record_daemon_log(
                &state,
                logging::LogLevel::Info,
                "session",
                format!(
                    "loaded Soulseek credentials for {} from {} credential store",
                    username, stored.source
                ),
            )
            .await;
        }
        Ok(None) => {}
        Err(error) => {
            record_daemon_log(
                &state,
                logging::LogLevel::Warn,
                "session",
                format!("could not load stored Soulseek credentials: {error}"),
            )
            .await;
        }
    }
    spawn_session_manager(Arc::clone(&state), session_receiver);
    spawn_search_expiry_scheduler(Arc::clone(&state));
    spawn_gold_star_club(Arc::clone(&state));
    spawn_vpn_polling(Arc::clone(&state));
    spawn_controller_config_watcher(Arc::clone(&state));
    relay_agent::spawn(Arc::clone(&state));
    spawn_download_auto_retry(Arc::clone(&state));
    spawn_download_auto_replace(Arc::clone(&state));
    spawn_transfer_rescue(Arc::clone(&state));
    spawn_backfill_scheduler(Arc::clone(&state));
    spawn_lidarr_sync_scheduler(Arc::clone(&state));
    spawn_source_discovery(Arc::clone(&state));
    spawn_mesh_dht_publisher(Arc::clone(&state));
    spawn_signal_shutdown_handler(Arc::clone(&state));
    spawn_configured_listeners(
        Arc::clone(&state),
        regular_listener_receiver,
        obfuscated_listener_receiver,
        shared_mesh_tcp,
    );
    spawn_bridge_server(Arc::clone(&state));
    spawn_rate_limit_cleanup(Arc::clone(&state));
    spawn_retention_scheduler(Arc::clone(&state));
    record_daemon_log(
        &state,
        logging::LogLevel::Debug,
        "daemon",
        "controller debug logging enabled",
    )
    .await;
    if let Some(gateway) = state.private_gateway.clone() {
        let gateway_state = Arc::clone(&state);
        state.spawn_managed_task(async move {
            if let Err(error) = gateway.run(gateway_state).await {
                ::tracing::error!(%error, "overlay gateway listener stopped");
            }
        });
    }
    if let Some(dht) = state.dht.clone() {
        state.spawn_managed_task(dht.run());
    }
    record_daemon_log(
        &state,
        logging::LogLevel::Info,
        "daemon",
        format!(
            "slskr {} instance {} listening on {} with log level {}",
            APP_VERSION,
            state.config.instance_name,
            address,
            logging::LogConfig::level_name(*state.log_level.read().await)
        ),
    )
    .await;

    if state.config.auto_connect {
        let credential_source = if state.runtime_credentials.read().await.is_some() {
            "stored"
        } else if state.config.credentials().is_some() {
            "config"
        } else {
            state.config.credential_store.as_str()
        };
        record_daemon_log(
            &state,
            logging::LogLevel::Info,
            "session",
            format!(
                "auto-connect enabled; queuing Soulseek connect using {credential_source} credentials"
            ),
        )
        .await;
        send_session_command(&state, SessionCommand::Connect).await?;
    } else {
        record_daemon_log(
            &state,
            logging::LogLevel::Info,
            "session",
            "auto-connect disabled; Soulseek connection will wait for an explicit connect request",
        )
        .await;
    }

    let listeners = match bind_http_listeners(&state.config.http_binds) {
        Ok(listeners) => listeners,
        Err(error) => {
            // Both frozen controller targets log HTTP bind failures as fatal
            // but return a successful process status from Program.Main.
            eprintln!("{error}");
            return Ok(());
        }
    };
    let http_connections = Arc::new(Semaphore::new(MAX_HTTP_CONNECTION_TASKS));
    #[cfg(unix)]
    if let Some(socket_path) = state.config.controller_web.socket.as_deref() {
        let unix_listener = bind_http_unix_listener(socket_path)?;
        println!("slskr listening on unix://{}", socket_path.display());
        let unix_state = Arc::clone(&state);
        let unix_connections = Arc::clone(&http_connections);
        state.spawn_managed_task(async move {
            loop {
                let Ok((stream, _)) = unix_listener.accept().await else {
                    break;
                };
                let connection_state = Arc::clone(&unix_state);
                let handler_state = Arc::clone(&connection_state);
                connection_state.spawn_bounded_http_task(&unix_connections, async move {
                    if let Err(error) = handle_http_stream(stream, None, false, handler_state).await
                    {
                        eprintln!("unix HTTP request failed: {error}");
                    }
                });
            }
        });
    }
    if !state.config.controller_web.https.disabled {
        let tls_acceptor = controller_tls_acceptor(&state.config.controller_web.https)?;
        let https_listeners = bind_http_listeners(&state.config.controller_web.https.binds)?;
        for (address, listener) in state
            .config
            .controller_web
            .https
            .binds
            .iter()
            .copied()
            .zip(https_listeners)
        {
            println!("slskr listening on https://{address}");
            let tls_state = Arc::clone(&state);
            let tls_connections = Arc::clone(&http_connections);
            let tls_acceptor = tls_acceptor.clone();
            state.spawn_managed_task(async move {
                loop {
                    let Ok((stream, remote_addr)) = listener.accept().await else {
                        break;
                    };
                    let acceptor = tls_acceptor.clone();
                    let connection_state = Arc::clone(&tls_state);
                    let handler_state = Arc::clone(&connection_state);
                    connection_state.spawn_bounded_http_task(&tls_connections, async move {
                        let Ok(stream) = acceptor.accept(stream).await else {
                            return;
                        };
                        if let Err(error) =
                            handle_http_stream(stream, Some(remote_addr), true, handler_state).await
                        {
                            eprintln!("HTTPS request failed: {error}");
                        }
                    });
                }
            });
        }
    }
    for address in &state.config.http_binds {
        println!("slskr listening on http://{address}");
    }
    if auth_disabled_on_non_loopback(&state.config) {
        eprintln!(
            "WARNING: HTTP API authentication is disabled on non-loopback bind {}; remote clients can control slskr",
            state.config.http_bind
        );
    } else if !state.config.auth_required {
        eprintln!(
            "WARNING: HTTP API authentication is disabled on {}; local processes can control slskr",
            state.config.http_bind
        );
    }
    let mut http_tasks = JoinSet::new();

    loop {
        while http_tasks.try_join_next().is_some() {}
        let accepts = listeners
            .iter()
            .map(|listener| Box::pin(listener.accept()))
            .collect::<Vec<_>>();
        let accepted = tokio::select! {
            command = lifecycle_receiver.recv(), if state.lifecycle_commands.is_some() => {
                match command {
                    Some(LifecycleCommand::Shutdown) => {
                        let _ = time::timeout(
                            GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT,
                            http_tasks.shutdown(),
                        )
                        .await;
                        state.shutdown_managed_tasks().await;
                        return Ok(());
                    }
                    Some(LifecycleCommand::Restart) => {
                        let _ = time::timeout(
                            GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT,
                            http_tasks.shutdown(),
                        )
                        .await;
                        state.shutdown_managed_tasks().await;
                        spawn_replacement_process()?;
                        return Ok(());
                    }
                    None => {
                        let _ = time::timeout(
                            GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT,
                            http_tasks.shutdown(),
                        )
                        .await;
                        state.shutdown_managed_tasks().await;
                        return Ok(());
                    }
                }
            }
            accepted = select_all(accepts) => accepted.0,
        };
        let (stream, _) = accepted.map_err(|error| format!("accept failed: {error}"))?;
        if once {
            let Ok(permit) = Arc::clone(&http_connections).try_acquire_owned() else {
                drop(stream);
                continue;
            };
            let _permit = permit;
            let result = handle_http_connection(stream, Arc::clone(&state)).await;
            state.shutdown_managed_tasks().await;
            return result;
        }
        let state = Arc::clone(&state);
        if !spawn_bounded_http_connection_task(&mut http_tasks, &http_connections, async move {
            if let Err(error) = handle_http_connection(stream, state).await {
                eprintln!("http request failed: {error}");
            }
        }) {
            continue;
        }
    }
}
