mod activitypub_controller;
mod app_state;
mod application_state;
#[allow(
    dead_code,
    reason = "batch compatibility types are retained for the public API surface"
)]
mod batch;
mod bloom_filter;
mod browse_path_state;
mod browse_runtime;
mod browse_store;
mod browse_wire;
mod cli;
mod collection_store;
mod config;
mod contact_state;
mod content_discovery;
mod controller_capabilities;
mod controller_cli;
mod controller_debug_view;
mod controller_feature_state;
mod controller_options_controller;
mod controller_options_projection;
mod controller_options_state;
mod controller_regex;
mod controller_release_check;
mod controller_route_inputs;
mod controller_reload;
mod controller_storage;
mod controller_storage_preflight;
mod controller_status;
mod controller_yaml;
mod credential_store;
mod daemon_runtime_setup;
mod daemon_serve;
mod database_maintenance;
mod destination_state;
mod dht;
mod discovery_graph;
mod distributed_runtime;
mod distributed_state;
mod dotnet_regex;
mod download_filter;
pub(crate) mod event_bus;
mod event_runtime;
mod event_store;
mod events_ws;
mod extended_controller;
mod feature_mutation_controller;
mod ftp;
mod hash_backfill_controller;
mod hash_backfill_runtime;
mod hash_backfill_state;
mod hash_db_store;
mod http_connection;
mod http_server;
mod incoming_share_store;
mod integration_runtime_state;
mod integration_target;
mod interest_store;
mod library_store;
mod library_controller;
mod lidarr_api;
mod lidarr_import;
mod lidarr_wishlist_sync;
mod listening_party_stream_state;
mod local_file_hash;
#[allow(
    dead_code,
    reason = "structured logging helpers are retained for optional runtime instrumentation"
)]
mod logging;
mod managed_blacklist_runtime;
mod managed_tasks;
mod mediacore_controller;
mod mesh_dht;
mod mesh_dht_runtime;
mod mesh_gateway_controller;
#[allow(
    dead_code,
    reason = "mesh transport security primitives are activated by optional mesh services"
)]
mod mesh_security;
mod mesh_services;
mod mesh_state;
mod mesh_sync;
mod message_store;
mod misc_controller_mutations;
mod multisource;
mod musicbrainz_controller;
mod musicbrainz_lookup;
mod native_compat_controller;
mod notification_runtime;
mod now_playing_store;
mod oauth_state;
#[allow(
    dead_code,
    reason = "OpenAPI generation is a supported developer surface"
)]
mod openapi;
#[allow(
    dead_code,
    reason = "persistence exposes operations used by optional compatibility routes"
)]
mod persistence;
mod pod_channels;
mod pod_membership_workflow;
mod podcore_controller;
mod podcore_runtime_stats;
mod pods;
mod port_forwarding;
mod preview_stream_controller;
mod preview_stream_state;
mod private_gateway;
mod private_message_auto_responses;
#[allow(
    dead_code,
    reason = "probe metadata builders are shared by optional live probes"
)]
mod probe_output;
mod quic_alpn;
mod ranking_controller;
#[allow(
    dead_code,
    reason = "rate-limit administration types are retained for API reporting"
)]
mod rate_limit;
mod request_input;
mod request_security;
mod realm_subject_index;
mod relay;
mod relay_agent;
mod relay_state;
mod relay_ws;
mod room_store;
mod runtime_compat_state;
mod share_scanner;
mod source_feed_ingest;
mod source_feed_preview_controller;
mod source_provider_catalog;
mod spotify_integration;
mod storage_directory_state;
mod swarm_analytics;
mod transfer_controller;
mod transfer_batch_controller;
mod transfer_completion;
mod user_note_store;
mod user_store;
mod wishlist_auto_download;
mod wishlist_csv_import;
mod wishlist_store;
mod wishlist_persistence;
// Shared route helpers are used by both the bounded dispatcher and the
// historical compatibility dispatcher.
mod file_transfer_runtime;
#[cfg(feature = "legacy-route-dispatch")]
mod legacy_route_dispatch;
mod listener_runtime;
mod listener_state;
mod peer_message_runtime;
mod peer_transport;
mod quarantine_controller;
mod record_list_filter;
mod route_dispatch_catalog;
mod route_dispatch;
mod route_request_entry;
#[allow(
    dead_code,
    reason = "routing response variants form the compatibility response surface"
)]
mod routing;
mod scripts;
mod search_fallback;
mod search_persistence;
mod search_runtime;
mod search_store;
mod security_controller;
#[allow(
    dead_code,
    reason = "bounded security controls are activated by optional security services"
)]
mod security_controls;
mod security_state;
mod session_runtime;
mod session_state;
mod share_grant_store;
mod share_group_store;
mod share_index_runtime;
mod share_index_state;
mod signalr_ws;
mod solid;
mod songid_controller;
mod songid_runtime;
mod songid_scoring;
mod soulfind_bridge_runtime;
mod source_discovery_runtime;
mod source_discovery_state;
#[allow(
    dead_code,
    reason = "storage codecs are retained for cache and compatibility formats"
)]
mod storage;
#[allow(
    dead_code,
    reason = "tracing context helpers are retained for optional instrumentation"
)]
mod tracing;
mod transfer_durability;
mod transfer_entry;
mod transfer_queue;
mod transfer_recovery;
mod transfer_recovery_runtime;
mod transfer_state_io;
mod upload_peer_cooldowns;
#[allow(
    dead_code,
    reason = "utility parsers support compatibility and test-only request shapes"
)]
mod utils;
mod versioned_get_contract;
mod versioned_relay_controller;
mod virtual_soulfind_v2;
mod virtual_soulfind_v2_controller;
mod vpn;
mod vpn_runtime;
mod web_static;
#[allow(
    dead_code,
    reason = "webhook verification and administration form the supported webhook surface"
)]
mod webhooks;

use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs},
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::SystemTime,
};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use futures_util::{StreamExt, future::select_all};
use rand::{TryRng, rngs::SysRng};
use serde::{Deserialize, Serialize};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use slskr_client::{
    ClientError,
    capabilities::{
        FEATURE_CAPABILITIES_V1, MAX_PEER_CAPABILITY_RECORDS, PeerCapabilityDescriptor,
        PeerCapabilityEnvelope, PeerCapabilityMessageType, decode_peer_capability_message,
        peer_capability_message,
    },
    connection::ConnectionKind,
    listener::{
        DEFAULT_INIT_HANDSHAKE_TIMEOUT, IncomingConnection, Listener, SharedIncomingConnection,
        demux_incoming, demux_obfuscated_incoming, demux_shared_incoming,
        demux_shared_mesh_incoming,
    },
    mesh::{MESH_RENDEZVOUS_INTEREST_TAG, MeshRendezvous, MeshRendezvousOptions},
    peer_connect::{send_obfuscated_peer_init, send_peer_init, send_pierce_firewall},
    protocol::{
        InitFrame, ProtocolTextEncoding, ROTATED_OBFUSCATION_TYPE, Reader, Writer,
        distributed::{DistributedMessage, DistributedSearch},
        peer::{
            FileAttribute, FileEntry, FileSearchResponse, FolderContentsRequest, PeerMessage,
            TransferRequest, TransferResponse, UserInfo,
        },
        server::{
            ConnectToPeerRequest, ConnectToPeerResponse, ObfuscatedPort, PeerAddress, RoomList,
            RoomListEntry, SearchRequest, ServerMessage, TargetedSearchRequest, UserStats,
            UserStatus, WaitPort, WatchedUser,
        },
    },
    search::{WishlistSearchScheduler, WishlistSearchSchedulerOptions},
    server::{LoginCredentials, ServerSession},
    share_payload::{compress_zlib_payload, decompress_zlib_payload},
    social::{MAX_PRIVATE_MESSAGE_RECIPIENTS, private_message_users_command},
    stream::{
        DistributedConnection, ObfuscatedPeerMessageConnection, PeerMessageConnection,
        ServerConnection,
    },
    version::{CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION, CLIENT_NAME},
};
use socket2::{Domain, Protocol, SockRef, Socket, Type};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        Mutex as AsyncMutex, OwnedSemaphorePermit, RwLock, Semaphore, broadcast, mpsc, oneshot,
        watch,
    },
    task::JoinSet,
    time::{self, Duration, Instant},
};

use self::app_state::{
    AppState, clear_connected_server_address, connected_server_address,
    effective_advertised_port, effective_controller_no_config_watch,
    effective_download_completed_path_template, effective_downloads_dir,
    effective_incomplete_dir, effective_obfuscated_advertised_port,
    effective_remote_configuration, effective_remote_file_management,
    effective_server_address, effective_user_info_description, effective_user_info_picture,
    reconfigure_obfuscated_listener, reconfigure_regular_listener,
};

pub(crate) use application_state::{
    ControllerVersionState, application_state_json_for_state, application_state_version_json,
    controller_version_json,
};
pub(crate) use event_bus::{
    publish_search_hub_event, publish_songid_hub_event, publish_transfer_hub_event,
};
pub(crate) use event_store::{EventRecord, EventStore};

#[cfg(any(
    feature = "full-controller-tests",
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
))]
pub(crate) use self::storage::write_file_atomic_with_temp_path;
pub(crate) use self::storage::{sync_state_directory, write_file_atomic};

use self::config::{MusicBrainzIntegrationSettings, PodSignatureMode, parse_compat_ip_address};
use self::daemon_runtime_setup::*;
use self::daemon_serve::serve;
use self::event_runtime::{
    persist_event_record_checked, record_daemon_log, record_event, record_http_log,
    record_soulseek_diagnostic,
};
use self::managed_tasks::{MAX_HTTP_CONNECTION_TASKS, spawn_bounded_http_connection_task};
use self::podcore_controller::dynamic_podcore_get_route;
use self::realm_subject_index::is_safe_opaque_reference;
use self::route_dispatch::route_http_request_with_state;
#[cfg(any(test, feature = "bounded-differential"))]
use self::route_request_entry::route_http_request_with_headers;
use self::route_dispatch_catalog::{
    audio_blank_recording_id_path, decoded_segments_after, extended_controller_dynamic_get_route,
    extended_controller_get_route, extended_controller_mutation_route,
};
use self::storage::SharedLocalFile;
use self::utils::*;
use self::wishlist_store::*;
use self::wishlist_persistence::{
    load_wishlist_store, persist_wishlist_ignored_result_and_searches_checked,
    persist_wishlist_ignored_result_delete_checked, persist_wishlist_item_checked,
    persist_wishlist_item_delete_checked, persist_wishlist_items_checked,
    rollback_wishlist_if_unchanged, wishlist_storage_error_response,
};

use self::activitypub_controller::{
    activitypub_actor_bound_to_signature, activitypub_actor_exists, activitypub_apply_relationship,
    activitypub_get_response, activitypub_webfinger_response, social_federation_is_active,
    verify_activitypub_inbox_signature,
};
#[cfg(feature = "bounded-differential")]
use self::activitypub_controller::{
    activitypub_public_key_pem, activitypub_signature_created_is_fresh,
    activitypub_signature_date_is_fresh, activitypub_signature_has_required_headers,
    decode_ed25519_pkix_public_key, parse_activitypub_signature_header,
};
use self::browse_path_state::{BrowseEntry, RemotePathEncodingRegistry};
use self::browse_runtime::{project_indirect_browse_response, project_peer_browse_response};
use self::browse_store::{
    BrowseRecord, BrowseStore, MAX_BROWSE_ENTRIES_PER_USER, MAX_BROWSE_EXTENSION_BYTES,
    MAX_BROWSE_FILENAME_BYTES, bounded_browse_entry, persist_browse_record_checked,
    rollback_browse_if_unchanged,
};
#[cfg(feature = "full-controller-tests")]
use self::browse_wire::build_empty_browse_payload;
use self::browse_wire::{
    MAX_BROWSE_WIRE_FILES_PER_RESPONSE, MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE,
    add_browse_wire_count, build_folder_contents_payload, build_shared_file_list_payload,
    group_browse_entries, group_share_entries, join_virtual_path, parse_folder_file_list_payload,
    parse_shared_file_list_payload, virtual_folder,
};
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::collection_store::CollectionItem;
use self::collection_store::{
    CollectionRecord, CollectionStore, persist_collection_checked, persist_collection_created,
    persist_collection_delete, rollback_collections_if_unchanged,
};
use self::contact_state::{
    ContactStore, ContactUpdateError, persist_contact_checked, persist_contact_delete_checked,
};
#[cfg(any(
    test,
    feature = "full-controller-tests",
))]
use self::controller_capabilities::new_capability_signing_key;
use self::controller_capabilities::{
    load_or_create_capability_signing_key, local_capability_descriptor,
    local_profile_peer_id, profile_friend_code,
    capabilities_negotiate_response, capabilities_parse_response, capabilities_response,
    capability_service_peer_json, native_capabilities_response,
    native_capability_controller_response, network_stats_value, persisted_capability_descriptor,
};
use self::controller_cli::{ControllerCliEnv, ServeInvocation, parse_serve_args};
use self::controller_debug_view::controller_options_debug_view;
#[cfg(feature = "bounded-differential")]
use self::controller_feature_state::ControllerFeatureStateFile;
use self::controller_feature_state::{ControllerFeatureState, ControllerFeatureStore};
use self::controller_options_controller::*;
use self::controller_options_projection::controller_options_json;
use self::controller_options_state::{
    ControllerOptionsOverlayState, ObfuscationReloadState, restart_reload_fingerprint,
};
#[cfg(feature = "full-controller-tests")]
use self::controller_regex::NATIVE_REGEX_MATCH_TIMEOUT;
use self::controller_regex::{
    ControllerRegex, compile_controller_regexes, compile_controller_regexes_for_request,
};
use self::controller_release_check::{
    controller_releases_url, controller_version_latest_response, start_controller_version_check,
};
#[cfg(feature = "bounded-differential")]
use self::controller_release_check::{
    is_newer_controller_release_available, normalize_controller_release_version,
    refresh_controller_version_check,
};
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
use self::controller_reload::load_watched_controller_configuration;
use self::controller_reload::{
    NATIVE_DEFAULT_ADVERSARIAL_YAML, apply_watched_controller_configuration,
    controller_compatibility_config_path, controller_options_config_location_json,
    read_controller_compatibility_yaml, spawn_controller_config_watcher,
    write_controller_compatibility_yaml,
};
#[cfg(all(feature = "full-controller-tests", unix))]
use self::controller_storage::controller_storage_directory_json_unix;
use self::controller_storage::{
    SLSKD_STORAGE_DIRECT_LIST_DEFAULT_ENTRIES, SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES,
    STORAGE_DIRECTORY_DELETE_DEPTH_ERROR, STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR,
    STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR, STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR,
    STORAGE_DIRECTORY_NOT_FOUND_ERROR, controller_share_directories_json,
    controller_storage_directory_json, controller_user_directories_json, controller_user_root_json,
    delete_scoped_file_storage_path, query_bool, query_bool_is_invalid,
    target_storage_directory_json,
};
#[cfg(feature = "full-controller-tests")]
use self::controller_storage::{
    SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES, SLSKD_STORAGE_MAX_RECURSION_DEPTH,
    SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES, SLSKD_STORAGE_RECURSIVE_LIST_MAX_ENTRIES,
    reserve_storage_scan_entry,
};
use self::controller_route_inputs::*;
use self::controller_status::*;
use self::controller_storage_preflight::{
    controller_events_read_failure_response,
    controller_native_backfill_candidates_read_failure_response,
    controller_native_backfill_file_write_failure_response,
    controller_native_events_read_failure_response, controller_native_hashdb_read_failure_response,
    controller_native_hashdb_write_failure_response, controller_native_search_query_validation,
    controller_native_search_storage_failure_response, controller_search_id_is_valid,
    controller_search_identifier_is_valid, controller_search_responses_read_failure_response,
};
use self::controller_yaml::{
    camel_to_snake_case, controller_options_config_body, controller_options_config_text_response,
    controller_yaml_api_projection, controller_yaml_target_validation_error, parse_controller_yaml,
};
use self::database_maintenance::{
    database_cleanup_value, database_stats_value, database_vacuum_value, spawn_retention_scheduler,
};
use self::destination_state::DestinationStore;
use self::distributed_runtime::{
    apply_distributed_settings, connect_distributed_parent, handle_embedded_distributed_search,
    hydrate_distributed_runtime, register_distributed_child,
    reset_distributed_network, run_distributed_persistence_worker,
};
#[cfg(feature = "full-controller-tests")]
use self::distributed_runtime::{
    handle_distributed_message, notify_distributed_branch, notify_distributed_child_depth,
};
use self::distributed_state::{
    DistributedConnectionRole, DistributedPersistenceSnapshot, DistributedPersistenceStatus,
    DistributedRuntime,
};
use self::extended_controller::{
    bridge_transfer_progress_response, controller_enqueue_download_batch,
    extended_controller_download_response, extended_controller_download_success_response,
    extended_controller_dynamic_get_response, extended_controller_get_response,
    extended_controller_mutation_response, listening_party_event_window,
    playback_priority_for_latest_feedback, prometheus_metric_json,
    transfer_batch_with_entries,
};
#[cfg(test)]
use self::extended_controller::virtual_soulfind_disaster_mode_level;
use self::feature_mutation_controller::{
    feature_controller_mutation_response, multisource_versioned_download_response,
    multisource_versioned_swarm_response, normalized_radar_timestamp, work_ref_is_recommendable,
    work_ref_search_text,
};
#[cfg(feature = "full-controller-tests")]
use self::file_transfer_runtime::apply_completed_download_permissions;
use self::file_transfer_runtime::{
    configured_download_destination_path, effective_transfer_group, effective_transfer_group_from,
    ensure_scoped_download_path, execute_accepted_file_transfer, fail_indirect_browse,
    fail_indirect_transfer, handle_inbound_file_transfer, inbound_upload_policy,
    open_download_file_for_read, prepare_transfer_local_path, probe_peer_capability,
    project_indirect_transfer_response, project_peer_transfer_response,
    render_configured_completed_download_path, safe_download_path, transfer_capacity_available,
    transfer_group_upload_settings,
};
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::file_transfer_runtime::{
    download_retry_delay, effective_download_pacing_limit, effective_upload_speed_limit,
    enforce_completed_download_content_safety, mp3_technical_metadata,
    prepare_incomplete_download_file, transfer_is_cancelled, update_transfer_progress,
    wav_technical_metadata, write_download_chunk_if_active,
};
use self::hash_backfill_controller::hashdb_backfill_from_history_response;
#[cfg(feature = "full-controller-tests")]
use self::hash_backfill_runtime::parse_flac_backfill_hash;
use self::hash_backfill_runtime::{
    backfill_candidates, backfill_file, receive_backfill_header, run_backfill_cycle,
    spawn_backfill_scheduler,
};
use self::hash_backfill_state::{
    BACKFILL_DEFAULT_CANDIDATES, BACKFILL_HASH_BYTES, BACKFILL_MAX_CANDIDATES,
    BACKFILL_MAX_HEADER_BYTES, BACKFILL_MAX_PER_PEER_PER_DAY, BACKFILL_MIN_IDLE_SECONDS,
    BACKFILL_RUN_INTERVAL_SECONDS, BackfillCandidate, BackfillState, PendingBackfillTransfer,
};
use self::hash_db_store::{
    hash_db_entry_from_persistence, hash_db_persistence_turn, hashdb_flac_inventory_key,
    hashdb_flac_inventory_record, hashdb_inventory_peer_ids, hashdb_inventory_records,
    hashdb_verification_entry_from_body, persist_current_hash_db_snapshot,
    persist_hash_db_snapshot, persisted_hash_db_record, read_hash_db_state_json,
    rollback_hash_db_entries_if_unchanged, write_hash_db_state_json,
    HASHDB_FLAC_INVENTORY_PREFIX,
};
use self::http_connection::{handle_http_connection, handle_http_stream};
use self::incoming_share_store::{IncomingShareRecord, IncomingShareStore};
use self::integration_runtime_state::{
    LidarrSyncRuntimeState, MAX_SOURCE_FEED_IMPORT_HISTORY, ProtectedSpotifyConnection,
    SourceFeedImportHistoryStore, SpotifyConnectionStore, SpotifySourceRow, SpotifySourceTarget,
};
use self::integration_target::{
    ResolvedIntegrationTarget, is_blocked_integration_ip, validate_integration_base_url,
    validate_lidarr_base_url,
};
use self::interest_store::{
    InterestStore, persist_interest_checked, persist_interest_delete_checked,
};
#[cfg(feature = "legacy-route-dispatch")]
use self::legacy_route_dispatch::legacy_route_http_request_with_headers_inner;
use self::library_controller::{
    find_shared_entry_for_content, library_browser_response, library_media_kind,
    native_library_items_search_json,
};
use self::library_store::{
    LibraryHealthIssueQuery, LibraryItemRecord, LibraryStore, persist_library_item_checked,
    persist_library_item_delete_checked, persist_library_items_checked, persisted_library_item,
    rollback_library_if_unchanged,
};
use self::lidarr_api::{
    fetch_lidarr_json_get, fetch_lidarr_manual_import_candidates, fetch_lidarr_system_status,
    fetch_lidarr_wanted_missing, lidarr_quality_profile_filter, start_lidarr_manual_import,
};
#[cfg(feature = "legacy-route-dispatch")]
use self::lidarr_import::import_lidarr_completed_directory;
use self::lidarr_import::{
    delete_lidarr_rejected_files, lidarr_import_history_key, list_lidarr_import_history,
    run_lidarr_automatic_import_with_history, run_lidarr_import_with_history,
};
#[cfg(feature = "full-controller-tests")]
use self::lidarr_import::{lidarr_map_import_path, lidarr_rejected_filenames};
#[cfg(feature = "full-controller-tests")]
use self::lidarr_wishlist_sync::run_lidarr_sync_scheduler_cycle;
use self::lidarr_wishlist_sync::{spawn_lidarr_sync_scheduler, sync_lidarr_wanted_to_wishlist};
use self::listener_runtime::spawn_configured_listeners;
use self::listener_state::{ListenerCommand, ListenerSnapshot, public_listener_error};
use self::listening_party_stream_state::{
    ListeningPartyStreamLimitRejection, ListeningPartyStreamLimits,
};
use self::local_file_hash::sha256_local_file_cached;
use self::managed_blacklist_runtime::ManagedBlacklistRuntime;
use self::managed_tasks::ManagedTaskRegistry;
use self::mediacore_controller::{mediacore_extended_response, mediacore_mutation_response};
use self::mesh_dht_runtime::{detect_nat_type, spawn_mesh_dht_publisher, STUN_SERVERS};
#[cfg(feature = "full-controller-tests")]
use self::mesh_dht_runtime::{STUN_MAGIC_COOKIE, parse_stun_mapped_address, stun_probe};
use self::mesh_gateway_controller::{
    mesh_gateway_auth_failure, mesh_gateway_disabled_response, mesh_http_service_response,
    mesh_http_services_response,
};
use self::mesh_state::MeshState;
use self::message_store::{
    MAX_MESSAGE_BODY_BYTES, MAX_MESSAGE_USERNAME_BYTES, MessageRecord, MessageStore,
    controller_conversation_read_failure_response, persist_conversation_delete_checked,
    persist_message_ack_checked, persist_message_acks_checked, persist_message_record_checked,
    persist_message_records_checked, rollback_messages_if_unchanged,
};
use self::misc_controller_mutations::misc_controller_mutation_response;
use self::musicbrainz_controller::{
    musicbrainz_dynamic_get_response, musicbrainz_mutation_response, radar_subscription_from_body,
    work_ref_wishlist_filter,
};
#[cfg(any(feature = "full-controller-tests", feature = "bounded-differential"))]
use self::musicbrainz_lookup::musicbrainz_release_target;
use self::musicbrainz_lookup::{
    MusicBrainzRecordingHit, fallback_podcore_metadata, musicbrainz_json_request,
    musicbrainz_query_encode, musicbrainz_release_target_with_settings,
    musicbrainz_search_recordings, parse_podcore_content_id, podcore_audio_metadata,
};
use self::native_compat_controller::{
    native_compat_response, native_compat_route, native_model_validation_response,
};
use self::notification_runtime::{
    send_private_message_notifications, send_room_mention_notifications,
};
#[cfg(feature = "full-controller-tests")]
use self::notification_runtime::{
    send_ntfy_notification_to, send_pushover_notification, send_pushbullet_notification,
};
use self::now_playing_store::{
    NowPlayingStore, native_nowplaying_webhook_response, persist_now_playing_checked,
    persist_now_playing_clear_checked,
};
use self::oauth_state::{
    OAuthStateRecord, OAuthStateStore, consume_oauth_state, load_oauth_state_store,
    persist_oauth_state_checked, rollback_oauth_states_if_unchanged, secure_oauth_state,
};
#[cfg(feature = "full-controller-tests")]
use self::oauth_state::secure_oauth_state_with;
use self::peer_message_runtime::{
    handle_obfuscated_peer_messages_with_address, handle_plain_peer_messages,
    handle_plain_peer_messages_with_address, schedule_queued_uploads, upload_queue_forecast,
};
#[cfg(feature = "full-controller-tests")]
use self::peer_message_runtime::{next_queued_upload_id, upload_queue_position};
#[cfg(all(test, feature = "focused-controller-tests"))]
use self::peer_transport::folder_entries_from_peer_message;
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::peer_transport::{
    OutboundPeerTransport, outbound_peer_dial_order, parse_folder_contents_response_payload,
};
use self::peer_transport::{
    PeerTransferNegotiation, SoulseekSocketClass, cached_peer_endpoint,
    configure_incoming_soulseek_socket, connect_file_transfer_preferred,
    connect_indirect_file_transfer, connect_soulseek_tcp, fetch_indirect_peer_browse,
    fetch_indirect_peer_folder, fetch_peer_browse, fetch_peer_folder, negotiate_peer_transfer,
    outgoing_peer_init_username, peer_connect_ip, remember_peer_endpoint, request_peer_endpoint,
    send_peer_message_oneway, send_peer_message_request, test_user_endpoint_peer_address,
};
use self::pod_membership_workflow::{
    PodJoinAcceptanceInput, PodJoinReplayStore, PodJoinSignatureInput, PodLeaveAcceptanceInput,
    PodLeaveRequestInput, PodMembershipWorkflowStore, pod_acceptor_has_permission,
    gold_star_club_available, pod_cancel_request_path, pod_local_peer_can_moderate,
    pod_pending_request_has_blank_id, pod_pending_request_path, pod_request_peer_id,
    verify_pod_join_signature, verify_pod_signed_payload,
};
use self::podcore_controller::{
    pod_channel_messages_path, pod_message_canonical_payload, pod_resource_segments,
    pod_verify_signature, podcore_dynamic_get_response, podcore_mutation_response,
    podcore_stats_response, route_pod_message_to_peer, trusted_mesh_peer_for,
    versioned_pods_blank_segment_response,
};
use self::podcore_runtime_stats::{PodCoreRuntimeStats, PodSignatureStats, PodVerificationStats};
use self::preview_stream_controller::{
    application_dump_request, create_preview_stream_ticket, find_shared_local_file,
    http_stream_ticket_path, issue_listening_party_stream_ticket, listening_party_stream_path,
    open_application_dump_file, open_local_preview_stream_file, open_primary_stream_file,
    open_remote_mesh_preview_file, open_remote_peer_preview_stream, open_shared_local_file,
    preview_stream_content_type,
    primary_stream_id, remote_preview_head_ticket, write_peer_preview_response,
    write_remote_preview_head_response,
};
use self::preview_stream_state::{
    PreviewStreamTicket, PreviewStreamTicketStore, open_preview_stream_ticket,
};
#[cfg(feature = "full-controller-tests")]
pub(crate) use self::private_message_auto_responses::MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS;
use self::private_message_auto_responses::{
    PrivateMessageAutoResponseTracker, is_private_message_auto_response_candidate,
};
#[cfg(feature = "full-controller-tests")]
use self::quarantine_controller::quarantine_verdict_payload_hash;
use self::quarantine_controller::{
    quarantine_build_audit_entry, quarantine_dynamic_get_response, quarantine_mutation_response,
};
use self::ranking_controller::{
    ranking_bad_request_response, ranking_history_counts, ranking_history_json,
    ranking_mutation_response, ranking_storage_failure_response,
};
use self::rate_limit::{controller_rate_limit_policy, spawn_rate_limit_cleanup};
use self::request_input::*;
use self::request_security::{
    authenticated_rate_limit_user_key, controller_cors_headers, mixed_websocket_auth_credentials,
    rate_limit_remote_addr, request_share_token, request_uses_revoked_jwt,
    websocket_auth_protocol,
    websocket_protocol_authorization,
};
#[cfg(feature = "full-controller-tests")]
use self::request_security::{
    parse_forwarded_element_ip, parse_forwarded_ip_token, rate_limit_user_key,
};
use self::record_list_filter::{RecordListFilter, parse_list_limit};
use self::relay_state::RelayState;
#[cfg(feature = "full-controller-tests")]
use self::room_store::RoomMessageRecord;
use self::room_store::{
    MAX_ROOM_USERNAME_BYTES, RoomRosterEntry, RoomStore, bounded_room_name,
};
use self::runtime_compat_state::{
    RuntimeCompatState, mutate_runtime_compat_state, mutate_runtime_compat_state_in_memory,
};
use self::search_persistence::{
    clear_persisted_searches, delete_persisted_search, delete_persisted_searches,
    normalize_search_status, persist_expired_searches_with_rollback, persist_search_record,
    persist_search_result_delta, persist_search_transition, persisted_search_record,
    persisted_search_result_records, persisted_search_status, persisted_target,
    rollback_search_record_if_unchanged, rollback_searches_if_unchanged,
    search_state_for_status, search_ttl_seconds_from_body,
};
use self::search_runtime::{
    build_file_search_response, search_shares, spawn_search_expiry_scheduler,
};
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::search_store::SearchCreateError;
use self::search_store::{
    SearchRecord, SearchResultEntry, SearchStore, bounded_search_result_entry,
    file_attribute_value, search_create_error_response,
};
use self::solid::{
    solid_client_id_document_response, solid_private_or_reserved, solid_resolution_error,
};
use self::security_controller::{
    native_adversarial_mutation_response, normalize_security_ban_value, security_ban_options,
    security_ban_route_tail, security_extended_response,
};
#[cfg(feature = "full-controller-tests")]
use self::security_state::MAX_SECURITY_BANS;
use self::security_state::{
    LoginAttemptStore, RevokedJwtStore, SecurityReputationProfile, SecurityState,
    persist_security_ban, persist_security_unban, rollback_security_ban_if_unchanged,
};
#[cfg(feature = "full-controller-tests")]
use self::session_runtime::handle_peer_message;
#[cfg(feature = "full-controller-tests")]
use self::session_runtime::{
    bridge_soulseek_room_message_to_pods, handle_incoming_soulseek_pod_message,
};
#[cfg(feature = "full-controller-tests")]
use self::session_runtime::{connect_session, handle_session_command, project_server_message};
use self::session_runtime::{
    is_remote_queue_response, persist_room_join_checked, persist_room_leave_checked,
    record_pod_room_mirror_failure, record_room_dispatch_failure, send_active_interest_command,
    send_room_join_if_connected, send_room_leave_if_connected, send_session_command,
    try_send_session_command, update_listeners, update_session,
};
use self::session_runtime::{spawn_gold_star_club, spawn_session_manager};
use self::session_state::{
    SearchDispatchTarget, SessionCommand, SessionSnapshot, public_session_error,
};
#[cfg(feature = "full-controller-tests")]
use self::share_grant_store::secure_share_grant_token_with;
use self::share_grant_store::{
    ShareAccessTokenRecord, ShareAccessTokenStore, ShareGrantStore, normalize_share_grant_username,
    persist_share_access_token, persist_share_grant, persist_share_grant_delete_checked,
    share_access_token_digest, share_grant_allows_download, share_grant_allows_reshare,
    share_grant_allows_stream,
};
use self::share_group_store::{
    ShareGroupMember, ShareGroupStore, persist_share_group, persist_share_group_delete,
    share_group_store_matches,
};
#[cfg(all(test, feature = "focused-controller-tests"))]
use self::share_index_runtime::commit_share_index_snapshot_checked;
use self::share_index_runtime::{
    add_runtime_share, cancel_active_share_scan, rebuild_share_index,
    rebuild_share_index_with_permit, share_rebuild_error_response,
};
#[cfg(feature = "full-controller-tests")]
use self::share_index_state::CatalogFilter;
use self::share_index_state::{
    ShareExtensionSummary, ShareIndexSnapshot, ShareLifecycleState, ShareRoot, pending_share_roots,
    persisted_share_file_records, public_share_cache_error, share_cache_path,
    write_share_cache_if_enabled,
};
#[cfg(feature = "bounded-differential")]
use self::share_scanner::extension_for;
#[cfg(feature = "full-controller-tests")]
use self::share_scanner::reserve_share_scan_entry;
#[cfg(test)]
use self::share_scanner::{ShareScanRequest, scan_share_dirs_with_cancellation};
use self::share_scanner::{
    build_share_index, build_share_index_with_cancellation, scan_share_dirs, summarize_extensions,
    virtual_share_path,
};
use self::songid_controller::{
    songid_capabilities_json, songid_evidence_package_json, songid_runs_value, songid_source_type,
};
use self::songid_runtime::{
    enqueue_songid_job, requeue_persisted_songid_runs, songid_local_file_is_allowed,
    songid_source_analysis, spawn_songid_workers,
};
#[cfg(feature = "full-controller-tests")]
use self::songid_runtime::{
    songid_acoustid_lookup, songid_extract_chromaprint, songid_fallback_query,
    songid_spotify_metadata_from_html,
};
#[cfg(feature = "full-controller-tests")]
#[allow(unused_imports)]
use self::soulfind_bridge_runtime::bridge_read_frame_with_timeout;
use self::soulfind_bridge_runtime::spawn_bridge_server;
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
#[allow(unused_imports)]
use self::soulfind_bridge_runtime::{
    BRIDGE_DOWNLOAD_REQUEST, BRIDGE_DOWNLOAD_RESPONSE, BRIDGE_LOGIN, BRIDGE_LOGIN_RESPONSE,
    BRIDGE_MAX_FRAME_BYTES, BRIDGE_ROOM_LIST_REQUEST, BRIDGE_ROOM_LIST_RESPONSE,
    BRIDGE_SEARCH_REQUEST, BRIDGE_SEARCH_RESPONSE, bridge_download_wire_response,
    bridge_handle_client, bridge_login_response, bridge_read_frame, bridge_read_i32,
    bridge_read_string, bridge_write_frame, bridge_write_i32, bridge_write_string,
};
use self::source_discovery_runtime::{
    dispatch_source_discovery_search, source_discovery_sources, spawn_source_discovery,
};
use self::source_discovery_state::{SOURCE_DISCOVERY_CYCLE_SECONDS, SourceDiscoveryState};
#[cfg(feature = "full-controller-tests")]
use self::source_feed_ingest::{MAX_SOURCE_PROVIDER_RESPONSE_BYTES, provider_metadata_row};
use self::source_feed_ingest::{
    build_provider_source_result, fetch_provider_metadata_page, loose_source_row, metadata_title,
    metadata_value, parse_simple_source_preview_items, preview_local_source_feed,
    provider_get_json, read_bounded_source_provider_bytes, read_bounded_source_provider_json,
    source_provider,
};
use self::source_feed_preview_controller::{
    preview_configured_provider_source_feed, preview_spotify_source_feed,
};
use self::spotify_integration::{
    build_spotify_source_result, complete_spotify_authorization, disconnect_spotify_connection,
    fetch_spotify_playlist_collection, fetch_spotify_playlist_tracks, fetch_spotify_track_pages,
    load_spotify_connection_store, looks_like_spotify_source, parse_spotify_source_target,
    spotify_callback_html, spotify_get_json, spotify_page_done, spotify_redirect_uri,
    spotify_source_access_token, spotify_track_row,
};
#[cfg(test)]
use self::spotify_integration::persist_spotify_connection_if_current;
#[cfg(any(test, feature = "full-controller-tests"))]
use self::spotify_integration::spotify_connection_path;
use self::source_provider_catalog::source_provider_catalog_json;
use self::storage_directory_state::{StorageDirectoryListOptions, StorageDirectoryListState};
use self::swarm_analytics::swarm_analytics_dashboard;
use self::transfer_batch_controller::*;
use self::transfer_completion::*;
use self::transfer_controller::{
    controller_accelerated_downloads_json, controller_download_stats_json,
    controller_download_user_stats_json, controller_native_autoreplace_mutation_response,
    controller_native_transfer_auto_replace_status_response,
    controller_native_transfer_input_validation_response,
    controller_native_transfer_storage_failure_response, controller_stuck_downloads_json,
    controller_telemetry_report_read_failure_response, controller_transfer_directories_report,
    controller_transfer_exceptions_pareto_report, controller_transfer_exceptions_report,
    controller_transfer_histogram_report, controller_transfer_leaderboard_report,
    controller_transfer_speeds_json, controller_transfer_storage_read_failure_response,
    controller_transfer_summary_report, controller_user_transfer_report,
    controller_versioned_transfer_histogram_report, controller_versioned_transfer_summary_report,
};
use self::transfer_durability::{
    database_i64, delete_persisted_transfers, next_transfer_updated_at_ms,
    persist_transfer_durability, persist_transfer_progress_record, persist_transfer_projection,
    persist_transfer_record, persist_transfer_records, record_transfer_rejection,
    remove_transfer_entries_if_unchanged, rollback_transfer_mutation_if_unchanged,
};
#[cfg(test)]
use self::transfer_durability::{persisted_transfer_event_record, persisted_transfer_record};
use self::transfer_entry::{
    TransferEntry, bounded_transfer_entry, bounded_transfer_filename, bounded_transfer_reason,
    bounded_transfer_username, download_request_projection, download_request_state,
    native_compatibility_download_json, native_download_status, public_transfer_events_error,
    public_transfer_reason, public_transfer_state_error, transfer_directory_name,
};
use self::transfer_queue::{TransferQueue, TransferRequestDetails};
#[cfg(feature = "full-controller-tests")]
use self::transfer_recovery::UnderperformanceReason;
use self::transfer_recovery::{
    AutoRetryTracker, RescueAction, RescueCandidate, RescueTracker, alternate_size_is_eligible,
    auto_retry_key, create_auto_retry_plan, create_rescue_plan, is_auto_retry_audio_file,
    prune_auto_retry_tracker,
};
#[cfg(feature = "full-controller-tests")]
use self::transfer_recovery_runtime::{auto_replace_retry_settings, run_transfer_rescue_cycle};
use self::transfer_recovery_runtime::{
    create_rescue_search, discover_mesh_range_sources, spawn_download_auto_replace,
    spawn_download_auto_retry, spawn_transfer_rescue,
};
#[cfg(feature = "full-controller-tests")]
use self::transfer_state_io::{
    append_transfer_event, load_transfer_state, open_transfer_event_file, write_transfer_state,
};
#[cfg(any(test, feature = "full-controller-tests"))]
use self::transfer_state_io::{transfer_events_path, transfer_state_path};
use self::upload_peer_cooldowns::{UploadPeerCooldowns, record_expected_upload_failure};
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::user_note_store::{MAX_USER_NOTE_BYTES, MAX_USER_NOTES};
use self::user_note_store::{
    UserNoteStore, persist_user_note_checked, persist_user_note_delete_checked,
};
use self::user_store::{
    MAX_USER_USERNAME_BYTES, UserRecord, UserStore, bounded_user_username,
    persist_user_projection,
};
use self::versioned_get_contract::versioned_get_failure_contract;
#[cfg(test)]
use self::versioned_relay_controller::persist_relay_share_database;
use self::versioned_relay_controller::{
    open_relay_controller_download, open_relay_controller_stream, relay_versioned_download_token,
    relay_versioned_route_allowed, relay_versioned_route_known,
    relay_versioned_stream_content_id, versioned_relay_request, versioned_relay_request_bytes,
};
use self::virtual_soulfind_v2_controller::route_virtual_soulfind_v2;
#[cfg(any(
    feature = "full-controller-tests",
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
))]
use self::virtual_soulfind_v2_controller::virtual_soulfind_catalogue;
use self::vpn_runtime::{ReconnectWake, spawn_vpn_polling, wait_for_reconnect_or_command};
#[cfg(all(test, feature = "focused-controller-tests"))]
use self::web_static::web_static_file_for_request;
use self::web_static::{
    is_spa_navigation_path, read_web_index_html, web_static_error_response,
    write_web_static_response,
};
#[cfg(feature = "bounded-differential")]
use self::web_static::{
    read_bounded_web_static_file, read_bounded_web_static_file_under_root,
    read_bounded_web_static_string, web_static_content_security_policy,
    web_static_file_for_request_under_root,
};
use self::webhooks::{
    dispatch_webhook_event, extract_webhook_events, persist_webhook_checked,
    persist_webhook_delete_checked, rollback_webhooks_if_unchanged, webhook_from_persisted,
};
use self::wishlist_auto_download::auto_download_completed_wishlist;
use self::wishlist_csv_import::{
    normalized_wishlist_csv_header, parse_simple_wishlist_import_rows, parse_wishlist_csv_rows,
    versioned_wishlist_csv_import_response, wishlist_csv_column,
};

use self::config::{
    AcoustIdIntegrationSettings, AppConfig, ChromaprintIntegrationSettings, ControllerProfile,
    IntegrationSettings, ShareDirectory, TrustedMeshPeer, json_bool_option, json_escape,
    json_option, json_u32_option, json_u64_option, json_usize_option, redact_username,
};
use self::routing::HttpResponse;
use self::utils::{
    RequestSecurityHeaders, cors_headers, is_active_transfer_status, is_failed_transfer_status,
    is_successful_transfer_status, is_terminal_transfer_status, non_empty, query_params,
    unix_timestamp, unix_timestamp_millis,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const SHARE_SCAN_CANCELLED_ERROR: &str = "share scan cancelled because settings changed";

pub fn run() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .expect("build slskr runtime");
    if let Err(error) = runtime.block_on(run_daemon()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run_daemon() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(invocation) = parse_serve_args(&args)? {
        return serve(invocation).await;
    }
    cli::run_from_args(args).await
}

static NEXT_DISTRIBUTED_PING_TOKEN: AtomicUsize = AtomicUsize::new(1);

fn next_distributed_ping_token() -> u32 {
    (NEXT_DISTRIBUTED_PING_TOKEN.fetch_add(1, Ordering::Relaxed) as u32).max(1)
}

fn print_controller_logo(_target: ControllerProfile) {
    print_native_product_logo();
}

fn print_native_product_logo() {
    let full_version = format!("{APP_VERSION} ({APP_VERSION})");
    let development = APP_VERSION == "0.0.0" || cfg!(debug_assertions);
    let padding = 56usize.saturating_sub(full_version.len());
    let padding_left = padding / 2;
    let padding_right = padding - padding_left;
    let centered_version = format!(
        "{}{full_version}{}",
        " ".repeat(padding_left),
        " ".repeat(padding_right)
    );
    let mut banner = format!(
        "\n\n                         slskR\n╒════════════════════════════════════════════════════════╕\n│           GNU AFFERO GENERAL PUBLIC LICENSE            │\n│                  https://github.com/snapetech/slskr    │\n│                                                        │\n│        native Soulseek client, daemon, and Web UI      │\n│{centered_version}│"
    );
    if development {
        banner.push_str("\n│■■■■■■■■■■■■■■■■■■■■► DEVELOPMENT ◄■■■■■■■■■■■■■■■■■■■■■│");
    }
    banner.push_str("\n└────────────────────────────────────────────────────────┘");
    println!("{banner}");
}

#[cfg(test)]
mod disaster_mode_regression_tests {
    #[test]
    fn status_starts_in_normal_mode_until_runtime_state_changes() {
        assert_eq!(super::virtual_soulfind_disaster_mode_level(), 0);
    }
}

#[cfg(test)]
mod mesh_sync_security_state_tests {
    #[test]
    fn sync_violation_state_bounds_peer_keys_and_reclaims_capacity() {
        let settings = crate::config::MeshSyncSecuritySettings {
            max_invalid_entries_per_window: 50,
            max_invalid_messages_per_window: 10,
            rate_limit_window: std::time::Duration::from_secs(300),
            quarantine_violation_threshold: 3,
            quarantine_duration: std::time::Duration::from_secs(1_800),
            proof_of_possession_enabled: false,
            require_signed_entries: false,
            consensus_min_peers: 5,
            consensus_min_agreements: 3,
            alert_threshold_signature_failures: 50,
            alert_threshold_rate_limit_violations: 20,
            alert_threshold_quarantine_events: 10,
        };
        let mut mesh = super::MeshState::new();
        for index in 0..super::MAX_MESH_SYNC_SECURITY_PEERS {
            mesh.sync_invalid_entries
                .insert(format!("peer-{index}"), (1_000, 1));
        }
        assert!(mesh.record_invalid_sync_entries("new-peer", 1, &settings, 1_000));
        assert_eq!(
            mesh.sync_invalid_entries.len(),
            super::MAX_MESH_SYNC_SECURITY_PEERS
        );
        assert!(mesh.record_invalid_sync_entries(
            &"x".repeat(super::MAX_MESH_SYNC_PEER_ID_BYTES + 1),
            1,
            &settings,
            1_000,
        ));

        assert!(!mesh.record_invalid_sync_entries("reclaimed-peer", 1, &settings, 2_000));
        assert_eq!(mesh.sync_invalid_entries.len(), 1);
        assert!(mesh.sync_invalid_entries.contains_key("reclaimed-peer"));
    }

    #[test]
    fn sync_violation_state_enforces_message_and_entry_limits_separately() {
        let mut settings = crate::config::MeshSyncSecuritySettings {
            max_invalid_entries_per_window: 50,
            max_invalid_messages_per_window: 2,
            rate_limit_window: std::time::Duration::from_secs(60),
            quarantine_violation_threshold: 10,
            quarantine_duration: std::time::Duration::from_secs(1_800),
            proof_of_possession_enabled: false,
            require_signed_entries: false,
            consensus_min_peers: 5,
            consensus_min_agreements: 3,
            alert_threshold_signature_failures: 50,
            alert_threshold_rate_limit_violations: 20,
            alert_threshold_quarantine_events: 10,
        };
        let mut mesh = super::MeshState::new();

        assert!(!mesh.record_invalid_sync_entries("peer", 1, &settings, 1_000));
        assert!(!mesh.record_invalid_sync_entries("peer", 1, &settings, 1_001));
        assert!(mesh.record_invalid_sync_entries("peer", 1, &settings, 1_002));
        assert_eq!(mesh.sync_invalid_entries["peer"].1, 3);
        assert_eq!(mesh.sync_invalid_messages["peer"].1, 3);

        settings.max_invalid_entries_per_window = 2;
        settings.max_invalid_messages_per_window = 50;
        let mut entry_limited = super::MeshState::new();
        assert!(!entry_limited.record_invalid_sync_entries("peer", 2, &settings, 1_000));
        assert!(entry_limited.record_invalid_sync_entries("peer", 1, &settings, 1_001));
        assert!(!entry_limited.record_invalid_sync_entries("peer", 1, &settings, 1_060));
    }
}

const MAX_WEBHOOK_DELIVERY_TASKS: usize = 32;
#[cfg(any(test, feature = "bounded-differential"))]
const MAX_INCOMING_CONNECTION_TASKS: usize = 128;
const MAX_PENDING_LISTENER_HANDSHAKES: usize = 128;
const MAX_SHARE_SCAN_TASKS: usize = 1;
const SHARE_SCAN_BUSY_ERROR: &str = "share scan already in progress";
const SHARE_SCAN_WORKER_ERROR: &str = "share scan worker failed";
const MAX_WEBSOCKET_CONNECTIONS: usize = 32;
const MAX_EXTERNAL_VISUALIZER_PROCESSES: usize = 4;
const MAX_PREVIEW_STREAMS: usize = 4;
const LISTED_PARTY_MAX_CONCURRENT_STREAMS: usize = 50;
const LISTED_PARTY_MAX_CONCURRENT_STREAMS_PER_IP: usize = 3;
const MAX_LISTED_PARTY_STREAM_LIMITERS: usize = 8_192;
const MAX_PREVIEW_STREAM_BYTES: u64 = 2 * 1024 * 1024 * 1024;

const MAX_ACTIVE_MESH_DISCOVERY_PROBES: usize = 8;

const PEER_CAPABILITY_LEASE_SECONDS: u64 = 24 * 60 * 60;

const MAX_POD_PENDING_MEMBERSHIP_RECORDS: usize = 4_096;

const POD_JOIN_TIMESTAMP_SKEW_MILLIS: u64 = 5 * 60 * 1_000;
const WEBSOCKET_AUTH_PROTOCOL_PREFIX: &str = "slskr.api-token.";
const MAX_MESH_SYNC_SECURITY_PEERS: usize = 4_096;
const MAX_MESH_SYNC_PEER_ID_BYTES: usize = 256;

const TRANSFER_PROGRESS_CHUNK_BYTES: usize = 64 * 1024;
const EVENT_HISTORY_LIMIT: usize = 500;
const MAX_EVENT_KIND_BYTES: usize = 128;
const MAX_EVENT_RESOURCE_BYTES: usize = 4 * 1024;
const MAX_EVENT_DETAIL_BYTES: usize = 64 * 1024;
const DEFAULT_LIST_LIMIT: usize = 500;
const DEFAULT_SEARCH_TTL_SECONDS: u64 = 15;
const DEFAULT_WISHLIST_SEARCH_TTL_SECONDS: u64 = 300;
const MAX_SEARCH_TTL_SECONDS: u64 = 24 * 60 * 60;
const MAX_WEB_STATIC_BYTES: u64 = 16 * 1024 * 1024;
const MAX_INTEGRATION_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

const MAX_SONGID_TOOL_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_PREVIEW_STREAM_TICKETS: usize = 1_000;
const MAX_SHARE_ACCESS_TOKENS: usize = 4_096;
const DEFAULT_SHARE_ACCESS_TOKEN_TTL_SECONDS: u64 = 30 * 24 * 60 * 60;
const MAX_SHARE_ACCESS_TOKEN_TTL_SECONDS: u64 = 365 * 24 * 60 * 60;
const SHARE_STREAM_TICKET_TTL_SECONDS: u64 = 120;
const MAX_CONTACT_RECORDS: usize = 4_096;
const MAX_CONTACT_STATUS_BYTES: usize = 64;
const MAX_SHARE_GROUPS: usize = 256;
const MAX_SHARE_GROUP_MEMBERS: usize = 4_096;
const MAX_TOTAL_SHARE_GROUP_MEMBERS: usize = 50_000;
const MAX_COLLECTIONS: usize = 256;
const MAX_COLLECTION_ITEMS: usize = 10_000;
const MAX_TOTAL_COLLECTION_ITEMS: usize = 50_000;
const MAX_WISHLIST_ITEMS: usize = 10_000;
const MAX_WISHLIST_IGNORED_RESULTS: usize = 50_000;
const MAX_WISHLIST_IGNORED_RESULTS_PER_ITEM: usize = 10_000;
const MAX_WISHLIST_FILTER_BYTES: usize = 4 * 1024;
const MAX_WISHLIST_RESULTS: usize = 10_000;
const MAX_WISHLIST_DOWNLOADS: u64 = 1_000_000;
const MAX_CSV_IMPORT_ROWS: usize = 10_000;

const MAX_LIST_NAME_BYTES: usize = 4 * 1024;
const MAX_LIST_DESCRIPTION_BYTES: usize = 16 * 1024;
const MAX_LIST_CONTENT_ID_BYTES: usize = 4 * 1024;
const MAX_LIST_ARTIST_BYTES: usize = 4 * 1024;
const MAX_LIST_TITLE_BYTES: usize = 4 * 1024;
const MAX_LIST_KIND_BYTES: usize = 256;
const MAX_LIBRARY_HEALTH_SCANS: usize = 256;
const MAX_LIBRARY_REMEDIATION_JOBS: usize = 256;

type UserInterestWaiter =
    oneshot::Sender<Result<slskr_client::protocol::server::UserInterests, String>>;
const MAX_SONGID_RUNS: usize = 256;

const MAX_USER_GROUP_BATCH: usize = 100;
const MAX_INTERESTS_PER_KIND: usize = 4_096;
const MAX_INTEREST_NAME_BYTES: usize = 4 * 1024;
const MAX_NOW_PLAYING_RECORDS: usize = 4_096;
const MAX_NOW_PLAYING_ARTIST_BYTES: usize = 4 * 1024;
const MAX_NOW_PLAYING_TITLE_BYTES: usize = 4 * 1024;

const MAX_SECURITY_BAN_USERNAME_BYTES: usize = MAX_USER_USERNAME_BYTES;
const MAX_SHARE_GRANTS: usize = 4_096;
const MAX_SHARE_GRANT_PERMISSIONS_BYTES: usize = 256;
const MAX_INCOMING_SHARES: usize = 4_096;
const MAX_INCOMING_SHARE_ITEMS: usize = 10_000;
const MAX_LIBRARY_ITEMS: usize = 10_000;
const MAX_CAPABILITY_NEGOTIATION_ITEMS: usize = 256;
const MAX_MEDIACORE_BATCH_ITEMS: usize = 100;
const MAX_MEDIACORE_PORTABILITY_ENTRIES: usize = 1_000;
const MAX_QUARANTINE_JURY_ITEMS: usize = 100;
const MAX_EXTENDED_DOWNLOAD_ITEMS: usize = 1_000;
const MAX_RANKING_BATCH_ITEMS: usize = 1_000;
const MAX_LISTENING_PARTY_TAGS: usize = 10;
const MAX_ROUTING_TARGET_PEERS: usize = 256;
const MAX_RADAR_MUTED_RELEASE_GROUPS: usize = 256;
const MAX_POD_DISCOVERY_TAGS: usize = 100;
const MAX_DESTINATIONS: usize = 256;
const MAX_SEARCH_RESULTS_PER_SEARCH: usize = 10_000;
const MAX_TOTAL_SEARCH_RESULTS: usize = 50_000;
const MAX_SEARCH_RECORDS: usize = 500;
const MAX_SEARCH_QUERY_BYTES: usize = 4 * 1024;
const MAX_SEARCH_TARGET_NAME_BYTES: usize = 1024;
const MAX_SEARCH_RESULT_USERNAME_BYTES: usize = 1024;
const MAX_SEARCH_RESULT_FILENAME_BYTES: usize = 4 * 1024;
const MAX_SEARCH_RESULT_EXTENSION_BYTES: usize = 256;

#[allow(dead_code)]
const APP_CAPABILITIES: &[&str] = &[
    "health",
    "version",
    "config",
    "stats",
    "metrics",
    "telemetry",
    "session-control",
    "session-privilege-check",
    "listeners",
    "shares",
    "share-catalog",
    "share-files",
    "share-rescan",
    "search-dispatch",
    "search-results",
    "user-watch",
    "user-stats",
    "user-browse",
    "messages",
    "rooms",
    "room-list-sync",
    "transfers",
    "events",
    "browser-session-auth",
    "csrf-origin-guard",
];

#[allow(dead_code)]
const NETWORK_CAPABILITIES: &[&str] = &[
    "server-session",
    "regular-listener",
    "obfuscated-listener",
    "plain-peer-messages",
    "obfuscated-peer-messages",
    "distributed-peer",
    "file-transfer",
    "indirect-connect",
];

#[allow(dead_code)]
const STORAGE_CAPABILITIES: &[&str] = &[
    "share-index-sqlite",
    "share-index-tsv",
    "transfer-events-sqlite",
    "transfer-events-tsv",
    "transfer-state-json",
];

const MAX_TRANSFER_STATE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRANSFER_EVENTS_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRANSFER_USERNAME_BYTES: usize = 1024;
const MAX_TRANSFER_FILENAME_BYTES: usize = 16 * 1024;
const MAX_TRANSFER_LOCAL_PATH_BYTES: usize = 16 * 1024;
const MAX_TRANSFER_BATCH_ID_BYTES: usize = 4 * 1024;
const MAX_TRANSFER_REQUEST_ID_BYTES: usize = 128;
const MAX_TRANSFER_REQUEST_NAME_BYTES: usize = 512;
const MAX_TRANSFER_METADATA_TEXT_BYTES: usize = 1024;
const MAX_TRANSFER_STATUS_BYTES: usize = 64;
const MAX_TRANSFER_REASON_BYTES: usize = 4 * 1024;
const MAX_TRANSFER_REQUEST_FILES: usize = 10_000;


#[allow(dead_code)]
const EXPERIMENTAL_CAPABILITIES: &[&str] = &[
    "direct-peer-browse",
    "direct-and-indirect-file-transfer",
    "dashboard",
];

fn frozen_obfuscation_startup_error(config: &AppConfig) -> Option<&'static str> {
    (!config.current_upstream_behavior
        && config.controller_profile == ControllerProfile::Native
        && config.obfuscation_enabled
        && !config.obfuscation_advertise_regular_port)
        .then_some(
            "Application terminated unexpectedly\nSystem.ArgumentException: The regular peer port must be advertised when peer obfuscation is enabled (Parameter 'advertiseRegularPort')",
        )
}

fn controller_share_value(root: &ShareRoot) -> serde_json::Value {
    let mut value = serde_json::json!({
        "localPath": root.local_path.display().to_string(),
        "id": share_root_id(&root.label),
        "alias": root.label,
        "raw": root.raw,
        "remotePath": root.label,
        "isExcluded": root.raw.starts_with(['!', '-']),
    });
    if root.statistics_ready {
        value["directories"] = serde_json::json!(root.directories);
        value["files"] = serde_json::json!(root.files);
    }
    value
}

const MAX_SHARE_SCAN_ENTRIES: usize = 131_072;
#[cfg(unix)]
const MAX_SHARE_SCAN_PENDING_DIRECTORIES: usize = 8192;
const SHARE_SCAN_ENTRY_LIMIT_ERROR: &str = "share scan stopped at aggregate directory entry limit";
#[cfg(unix)]
const SHARE_SCAN_PENDING_DIRECTORY_LIMIT_ERROR: &str =
    "share scan stopped at pending directory limit";

fn virtual_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn stable_content_hash(path: &str, size: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.as_bytes().iter().copied().chain(size.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

const HASHDB_BACKFILL_PROGRESS_KEY: &str = "hashdb/backfill/progress";
const HASHDB_BACKFILL_MIN_FILE_SIZE: u64 = 32_768;
const HASHDB_SCHEMA_VERSION: i64 = 24;

fn truncate_utf8_bytes(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

fn bounded_event_detail(detail: Option<String>) -> Option<String> {
    detail.map(|detail| {
        if detail.len() <= MAX_EVENT_DETAIL_BYTES {
            detail
        } else {
            format!("<omitted oversized event detail: {} bytes>", detail.len())
        }
    })
}

#[cfg(feature = "legacy-route-dispatch")]
async fn controller_remove_transfer_file_if_present(path: &str) -> Result<(), String> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("transfer file removal failed: {error}")),
    }
}

fn controller_transfer_state(status: &str) -> &str {
    match status {
        "queued" => "Queued",
        "accepted" | "peer_lookup" | "peer_negotiating" | "indirect_pending" | "in_progress" => {
            "InProgress"
        }
        "succeeded" | "completed" => "Completed",
        "cancelled" => "Cancelled",
        "failed" | "rejected" | "errored" => "Failed",
        other => other,
    }
}

async fn content_discovery_error_response(state: &AppState, error: String) -> HttpResponse {
    if error.starts_with("content discovery state ") {
        update_session(state, |snapshot| {
            snapshot.last_error = Some(error);
        })
        .await;
        routing::service_unavailable_response("content discovery storage is unavailable")
    } else {
        routing::bad_request_response(&error)
    }
}

// Collection Models
fn unix_seconds_rfc3339(timestamp: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(i64::try_from(timestamp).unwrap_or(i64::MAX), 0)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn bridge_started_at_string(timestamp: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(i64::try_from(timestamp).unwrap_or(i64::MAX), 0)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

/// Matches the oracle's real per-owner Collections/Share-Grants scoping:
/// a caller with a real resolvable identity (`Some`) may not see/mutate a
/// resource owned by a *different* real identity. An empty
/// `owner_user_id` (legacy data, or created before any per-caller
/// identity existed) and a `None` caller (no resolvable identity at all)
/// both preserve today's unrestricted behavior.
fn collection_owner_forbids(caller_id: Option<&str>, owner_user_id: &str) -> bool {
    caller_id.is_some_and(|caller_id| !owner_user_id.is_empty() && owner_user_id != caller_id)
}

/// Matches the oracle's real Share-Grants ownership gate (SharesController.cs):
/// a share grant is owned transitively through its collection, so every
/// grant action checks `collection.OwnerUserId == currentUserId`, treating
/// a mismatch identically to the collection/grant not existing (`NotFound`).
async fn share_grant_collection_forbids(
    state: &AppState,
    collection_id: &str,
    caller_id: Option<&str>,
) -> bool {
    state
        .collections
        .read()
        .await
        .get(collection_id)
        .is_some_and(|collection| collection_owner_forbids(caller_id, &collection.owner_user_id))
}

struct SongIdJob {
    run_id: String,
    source: String,
    source_type: String,
    requested_query: Option<String>,
    match_query: bool,
}

/// Return the last validated download policy, including changes applied by
/// the watched YAML configuration.  `AppConfig` is intentionally immutable
/// after startup for compatibility and diagnostics, while runtime consumers
/// must observe a successful live configuration reload immediately.
async fn effective_download_exclusions(state: &AppState) -> Vec<String> {
    state
        .options_overlay
        .read()
        .await
        .watched_download_exclusions
        .clone()
        .unwrap_or_else(|| state.config.download_filter.exclude.clone())
}

async fn cancel_download_if_blocked_by_policy(
    state: &AppState,
    transfer: &TransferEntry,
) -> Option<String> {
    if transfer.direction != 0 || is_terminal_transfer_status(&transfer.status) {
        return None;
    }
    let exclusions = effective_download_exclusions(state).await;
    let exclusion = download_filter::matching_exclusion(&transfer.filename, &exclusions)?;
    let reason = format!("blocked by download exclusion: {exclusion}");
    let updated = {
        let mut transfers = state.transfers.write().await;
        let current = transfers
            .entries
            .iter()
            .find(|entry| entry.id == transfer.id)?;
        if is_terminal_transfer_status(&current.status) {
            return None;
        }
        transfers.update_status(transfer.id, "cancelled", None, Some(reason.clone()))
    };
    if let Some(updated) = updated {
        persist_transfer_projection(state, &updated).await;
        record_event(
            state,
            "downloads.policy_cancelled",
            transfer.id.to_string(),
            Some(reason.clone()),
        )
        .await;
        return Some(reason);
    }
    None
}

async fn effective_sanitized_config_json(state: &AppState) -> String {
    let exclusions = effective_download_exclusions(state).await;
    let sanitized = state.config.sanitized_json();
    let mut value = serde_json::from_str::<serde_json::Value>(&sanitized)
        .unwrap_or_else(|_| serde_json::json!({}));
    value["filters"]["download"]["exclude"] = serde_json::json!(exclusions);
    serde_json::to_string(&value).unwrap_or(sanitized)
}

async fn cancel_downloads_blocked_by_policy(state: &AppState, exclusions: &[String]) {
    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers
            .entries
            .iter()
            .filter(|entry| {
                entry.direction == 0
                    && !is_terminal_transfer_status(&entry.status)
                    && download_filter::is_excluded(&entry.filename, exclusions)
            })
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|id| {
                let exclusion = transfers
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .and_then(|entry| {
                        download_filter::matching_exclusion(&entry.filename, exclusions)
                    })
                    .unwrap_or_else(|| "global download policy".to_owned());
                transfers.update_status(
                    id,
                    "cancelled",
                    None,
                    Some(format!("blocked by download exclusion: {exclusion}")),
                )
            })
            .collect::<Vec<_>>()
    };
    if updated.is_empty() {
        return;
    }
    let count = updated.len();
    if let Err(error) = persist_transfer_records(state, &updated).await {
        record_daemon_log(
            state,
            logging::LogLevel::Error,
            "configuration",
            format!("failed to persist policy-cancelled downloads: {error}"),
        )
        .await;
    }
    record_event(
        state,
        "downloads.policy_cancelled",
        "downloads",
        Some(format!("count={count}")),
    )
    .await;
}

/// Matches the oracle's real `PeerReputation.TrustedThreshold`/
/// `UntrustedThreshold`/`MaxScore`.
const SECURITY_REPUTATION_TRUSTED_THRESHOLD: i32 = 70;
const SECURITY_REPUTATION_UNTRUSTED_THRESHOLD: i32 = 20;
const SECURITY_REPUTATION_DEFAULT_SCORE: i32 = 50;

fn webhook_resource_id<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let id = path.strip_prefix(prefix)?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

fn webhook_test_id<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let path = path.strip_prefix(prefix)?;
    let id = path.strip_suffix("/test")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

// Share Grant Models

fn share_grant_resource_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/share-grants/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

fn share_grant_collection_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/share-grants/by-collection/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

fn share_grant_helper_id<'a>(path: &'a str, helper: &str) -> Option<&'a str> {
    let path = path.strip_prefix("/api/share-grants/")?;
    let id = path.strip_suffix(&format!("/{helper}"))?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

fn share_grant_manifest_id(path: &str) -> Option<&str> {
    share_grant_helper_id(path, "manifest")
}

fn share_stream_content_id(path: &str) -> Option<String> {
    let path = path.strip_prefix("/api/streams/")?;
    let content_id = path.strip_suffix("/share-ticket")?;
    (!content_id.is_empty() && !content_id.contains('/')).then(|| decoded_path_segment(content_id))
}

fn e2e_share_announce_enabled() -> bool {
    std::env::var("SLSKDN_E2E_SHARE_ANNOUNCE")
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn lidarr_missing_albums_value(library: &LibraryStore) -> Vec<serde_json::Value> {
    library
        .health_issues()
        .into_iter()
        .map(|issue| {
            let item_id = issue
                .get("item_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            serde_json::json!({
                "id": format!("lidarr-{item_id}"),
                "artist": issue.get("artist").cloned().unwrap_or_else(|| serde_json::json!("")),
                "title": issue.get("title").cloned().unwrap_or_else(|| serde_json::json!("")),
                "issueType": issue.get("type").cloned().unwrap_or_else(|| serde_json::json!("unknown")),
                "source": "library-health",
                "status": "missing_metadata",
                "item_id": item_id,
            })
        })
        .collect()
}

// Destination Models

/// Resolve an absolute path through every existing ancestor and append any
/// missing tail. This preserves validation for destinations that will be
/// created later while still following symlinks in existing ancestors. Any
/// non-NotFound filesystem error is rejected by returning `None`.
fn canonicalize_with_missing_tail(path: &Path) -> Option<PathBuf> {
    let mut existing = path;
    let mut missing = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(mut canonical) => {
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Some(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name()?.to_os_string();
                missing.push(name);
                existing = existing.parent()?;
            }
            Err(_) => return None,
        }
    }
}

fn normalize_absolute_path(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    normalized.is_absolute().then_some(normalized)
}

fn directory_is_writable(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let probe = path.join(format!(
        ".slskr-write-test-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let writable = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    if writable {
        let _ = fs::remove_file(probe);
    }
    writable
}

async fn record_distributed_persistence_failure(state: &AppState, operation: &str, error: String) {
    record_daemon_log(
        state,
        logging::LogLevel::Error,
        "distributed",
        format!("{operation}: {error}"),
    )
    .await;
}

fn resolve_external_visualizer_path(configured: Option<&str>) -> Option<PathBuf> {
    let configured = configured
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let path = Path::new(configured);
    if path.is_absolute() || path.components().count() > 1 {
        return path
            .is_file()
            .then(|| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(configured))
        .find(|candidate| candidate.is_file())
}

fn path_directories() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .collect()
}

fn command_exists_on_path(command: &str) -> bool {
    path_directories()
        .iter()
        .any(|directory| directory.join(command).is_file())
}

fn configured_command_exists(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    let path = Path::new(command);
    if path.is_absolute() || path.components().count() > 1 {
        path.is_file()
    } else {
        command_exists_on_path(command)
    }
}

fn file_exists_at_known_location_or_on_path(candidates: &[&str], path_file_name: &str) -> bool {
    candidates
        .iter()
        .any(|candidate| Path::new(candidate).is_file())
        || command_exists_on_path(path_file_name)
}

/// Formats a duration the way .NET's default `TimeSpan.ToString()` does:
/// `d.hh:mm:ss` once it spans a full day, else `hh:mm:ss`.
fn format_timespan_hms(total_seconds: i64) -> String {
    let total_seconds = total_seconds.max(0);
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    if days > 0 {
        format!("{days}.{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

/// Formats a whole-millisecond duration using the .NET `TimeSpan` JSON
/// representation. Preserve the fractional component when a target stats
/// service reports a sub-second average; the older h:m:s helper intentionally
/// remains unchanged for endpoints whose contract only exposes seconds.
fn format_timespan_millis(total_millis: u64) -> String {
    let total_seconds = total_millis / 1_000;
    let milliseconds = total_millis % 1_000;
    let base = format_timespan_hms(i64::try_from(total_seconds).unwrap_or(i64::MAX));
    if milliseconds == 0 {
        base
    } else {
        // TimeSpan's constant format uses seven fractional decimal places;
        // millisecond counters contribute the first three and four trailing
        // zeroes.
        format!("{base}.{milliseconds:03}0000")
    }
}

fn resolve_external_visualizer_working_directory(
    configured: Option<&Path>,
    resolved_path: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(configured) = configured {
        return configured
            .is_dir()
            .then(|| fs::canonicalize(configured).unwrap_or_else(|_| configured.to_path_buf()));
    }
    resolved_path.and_then(Path::parent).map(Path::to_path_buf)
}

fn controller_forbidden_response() -> HttpResponse {
    HttpResponse {
        status: "403 Forbidden",
        content_type: "",
        body: String::new(),
    }
}

fn controller_options_validation_failure_response(state: &AppState) -> Option<HttpResponse> {
    let error = state
        .controller_options_validation_error
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    error.as_ref()?;
    Some(match state.config.controller_profile {
        ControllerProfile::Legacy => HttpResponse {
            status: "500 Internal Server Error",
            content_type: "application/json; charset=utf-8",
            body: serde_json::Value::String("A validation error has occurred.".to_owned())
                .to_string(),
        },
        ControllerProfile::Native => HttpResponse {
            status: "500 Internal Server Error",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "title": "Internal Server Error",
                "status": 500,
                "detail": "An unexpected error occurred.",
                "traceId": "0H00000000000:00000001",
            })
            .to_string(),
        },
    })
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifecycleCommand {
    Shutdown,
    Restart,
}

const GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

fn schedule_lifecycle_command(state: &AppState, command: LifecycleCommand) {
    let Some(sender) = state.lifecycle_commands.clone() else {
        return;
    };
    tokio::spawn(async move {
        // Let the HTTP response flush before the accept loop tears down the runtime.
        time::sleep(Duration::from_millis(100)).await;
        let _ = sender.send(command).await;
    });
}

/// Disconnects from the Soulseek server (best-effort) before scheduling
/// process shutdown, matching the oracle's `StopAsync` teardown
/// (`Client.Disconnect("Shutting down", ...)`) rather than exiting with the
/// session left connected.
async fn initiate_graceful_shutdown(state: &AppState) {
    cancel_active_share_scan(state);
    // A full session command queue must not prevent the process from honoring
    // SIGTERM/SIGINT. The lifecycle command still closes the listeners after
    // the bounded best-effort disconnect window.
    let _ = time::timeout(
        GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT,
        send_session_command(state, SessionCommand::Disconnect),
    )
    .await;
    schedule_lifecycle_command(state, LifecycleCommand::Shutdown);
}

#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
fn public_lidarr_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|_| "Lidarr connection failed")
}

fn index_html_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: index_html(),
    }
}

fn fallback_dashboard_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: fallback_dashboard_html(),
    }
}

fn head_response(response: HttpResponse) -> HttpResponse {
    // Keep the GET representation so the HTTP writer can advertise its
    // length for HEAD while suppressing the body bytes on the wire.
    response
}

fn web_build_root(
    configured_content_path: Option<&Path>,
    runtime_profile: Option<ControllerProfile>,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(configured) = configured_content_path {
        candidates.push(configured.to_path_buf());
    }
    if let Ok(configured) = env::var("SLSKR_WEB_BUILD_DIR") {
        candidates.push(PathBuf::from(configured));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("web").join("build"));
            candidates.push(exe_dir.join("build"));
            candidates.push(
                exe_dir
                    .join("..")
                    .join("share")
                    .join("slskr")
                    .join("web")
                    .join("build"),
            );
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("web").join("build"));
        candidates.push(cwd.join("build"));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("web")
            .join("build"),
    );

    if let Some(target) = runtime_profile {
        // Prefer a profile build across every candidate root before falling
        // back to a legacy single-build root. The executable's generated
        // fallback shell can otherwise win before SLSKR_WEB_BUILD_DIR is
        // considered, silently defeating the selected compatibility UI.
        if let Some(profile_root) = candidates.iter().find_map(|path| {
            let profile_root = path.join(target.as_str());
            profile_root
                .join("index.html")
                .is_file()
                .then_some(profile_root)
        }) {
            return Some(profile_root);
        }
    }

    candidates
        .into_iter()
        .find(|path| path.join("index.html").is_file())
}

fn constant_time_bytes_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let maximum = left.len().max(right.len());
    for index in 0..maximum {
        difference |= usize::from(left.get(index).copied().unwrap_or_default())
            ^ usize::from(right.get(index).copied().unwrap_or_default());
    }
    difference == 0
}

async fn read_bounded_integration_json(
    response: reqwest::Response,
    label: &str,
) -> Result<serde_json::Value, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_INTEGRATION_RESPONSE_BYTES as u64)
    {
        return Err(format!(
            "{label} response exceeds {MAX_INTEGRATION_RESPONSE_BYTES} bytes"
        ));
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("{label} response read failed: {error}"))?;
        if body.len().saturating_add(chunk.len()) > MAX_INTEGRATION_RESPONSE_BYTES {
            return Err(format!(
                "{label} response exceeds {MAX_INTEGRATION_RESPONSE_BYTES} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }

    serde_json::from_slice(&body).map_err(|error| format!("invalid {label} JSON: {error}"))
}

#[cfg(any(test, feature = "bounded-differential"))]
async fn route_http_request(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
) -> Result<HttpResponse, String> {
    Box::pin(route_http_request_with_headers(
        method,
        path,
        authorization,
        body,
        state,
        RequestSecurityHeaders {
            remote_addr: Some("127.0.0.1:1".parse().expect("loopback test address")),
            ..RequestSecurityHeaders::default()
        },
    ))
    .await
}

fn unversioned_mutation_requires_api_version(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("POST", "/api/audio/analyzers/migrate")
            | ("POST", "/api/bridge/search")
            | ("POST", "/api/bridge/download")
            | ("POST", "/api/bridge/start")
            | ("POST", "/api/bridge/stop")
            | ("PUT", "/api/bridge/admin/config")
            | ("POST", "/api/jobs/mb-release")
            | ("POST", "/api/jobs/discography")
            | ("POST", "/api/jobs/label-crate")
            | ("POST", "/api/library/health/scans")
            | ("POST", "/api/library/health/issues/fix")
            | ("POST", "/api/source-feed-imports/preview")
            | ("POST", "/api/hashdb/backfill/from-history")
            | ("POST", "/api/integrations/spotify/authorize")
            | ("DELETE", "/api/integrations/spotify")
    ) || (method == "PATCH"
        && path
            .strip_prefix("/api/library/health/issues/")
            .is_some_and(|issue_id| !issue_id.is_empty() && !issue_id.contains('/')))
}

fn download_request_path(path: &str) -> Option<(&str, Option<&str>)> {
    let rest = path
        .strip_prefix("/api/v0/downloads/requests/")
        .or_else(|| path.strip_prefix("/api/downloads/requests/"))?;
    let (id, action) = rest
        .split_once('/')
        .map_or((rest, None), |(id, action)| (id, Some(action)));
    (!id.is_empty()
        && !id.contains('/')
        && uuid::Uuid::parse_str(id).is_ok()
        && action.is_none_or(|action| !action.is_empty() && !action.contains('/')))
    .then_some((id, action))
}

fn decoded_path_segment(segment: &str) -> String {
    percent_decode_component(segment)
}

const PODCORE_MIN_DATETIME: &str = "0001-01-01T00:00:00+00:00";

fn conversation_message_path(path: &str) -> Option<(&str, u64)> {
    let rest = path.strip_prefix("/api/conversations/")?;
    let (username, id) = rest.split_once('/')?;
    if username.is_empty() || username.contains('/') || id.contains('/') {
        return None;
    }
    Some((username, id.parse().ok()?))
}

fn versioned_conversation_mutation_validation_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    let rest = path.strip_prefix("/api/v0/conversations/")?;
    let segments = rest.split('/').collect::<Vec<_>>();
    match (method, segments.as_slice()) {
        ("PUT", [username, id]) => {
            if decoded_path_segment(username).trim().is_empty() {
                return Some(routing::bad_request_response("username is required"));
            }
            if !id.parse::<i32>().map(|value| value > 0).unwrap_or(false) {
                return Some(routing::bad_request_response("id must be positive"));
            }
        }
        ("PUT", [username]) | ("DELETE", [username])
            if decoded_path_segment(username).trim().is_empty() =>
        {
            return Some(routing::bad_request_response("username is required"));
        }
        _ => {}
    }
    None
}

fn conversation_messages_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/conversations/")?
        .strip_suffix("/messages")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

struct LocalStreamFile {
    file: fs::File,
    length: u64,
    content_type: String,
    cleanup_path: Option<PathBuf>,
}

fn search_target_static(target: &str) -> &'static str {
    match target {
        "user" => "user",
        "room" => "room",
        "wishlist" => "wishlist",
        _ => "global",
    }
}

async fn record_peer_security_violation(state: &AppState, username: &str) {
    let settings = state.advanced_networking.read().await.security.clone();
    let banned = state
        .security
        .write()
        .await
        .record_peer_violation(username, &settings);
    if banned {
        record_daemon_log(
            state,
            logging::LogLevel::Warn,
            "security",
            format!(
                "peer auto-banned after configured violation threshold: {}",
                redact_username(username)
            ),
        )
        .await;
    }
}

fn restore_changed_value_if_unchanged<T: Clone + PartialEq>(
    current: &mut T,
    previous: &T,
    mutated: &T,
) {
    if previous != mutated && current == mutated {
        *current = previous.clone();
    }
}

pub fn index_html() -> String {
    if let Some(html) = read_web_index_html() {
        return html;
    }

    fallback_dashboard_html()
}

pub fn fallback_dashboard_html() -> String {
    web_static::fallback_dashboard_html()
}

fn share_root_id(alias: &str) -> String {
    let mut digest = <Sha1 as Sha1Digest>::new();
    Sha1Digest::update(&mut digest, alias.as_bytes());
    hex::encode_upper(Sha1Digest::finalize(digest))
}

fn escape_cache_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    append_escaped_cache_field(&mut escaped, value);
    escaped
}

fn append_escaped_cache_field(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            _ => output.push(character),
        }
    }
}

#[cfg(any(
    feature = "full-controller-tests",
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
))]
#[allow(dead_code, unused_imports, unused_variables)]
#[path = "controller_tests.rs"]
pub mod tests;

#[cfg(all(test, feature = "focused-controller-tests"))]
#[path = "focused_controller_tests.rs"]
mod focused_controller_tests;

#[cfg(any(
    feature = "bounded-protocol-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4",
    feature = "bounded-persistence-tests",
    feature = "bounded-file-lifecycle-tests",
    feature = "bounded-security-control-tests",
    feature = "bounded-security-authorization-tests"
))]
pub fn run_bounded_differential() {
    tests::run_bounded_differential_tests();
}

#[cfg(not(any(
    feature = "bounded-protocol-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4",
    feature = "bounded-persistence-tests",
    feature = "bounded-file-lifecycle-tests",
    feature = "bounded-security-control-tests",
    feature = "bounded-security-authorization-tests"
)))]
pub fn run_bounded_differential() {
    panic!("no bounded differential feature selected");
}
