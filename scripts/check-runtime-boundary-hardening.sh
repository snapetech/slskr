#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source_files=(
  crates/slskr/src/lib.rs
  crates/slskr/src/activitypub_controller.rs
  crates/slskr/src/session_runtime.rs
  crates/slskr/src/session_runtime_owners/*.rs
  crates/slskr/src/security_controller.rs
  crates/slskr/src/quarantine_controller.rs
  crates/slskr/src/misc_controller_mutations.rs
  crates/slskr/src/native_compat_controller.rs
  crates/slskr/src/feature_mutation_controller.rs
  crates/slskr/src/controller_reload.rs
  crates/slskr/src/controller_cli.rs
  crates/slskr/src/musicbrainz_controller.rs
  crates/slskr/src/virtual_soulfind_v2_controller.rs
  crates/slskr/src/daemon_serve.rs
  crates/slskr/src/soulfind_bridge_runtime.rs
  crates/slskr/src/legacy_route_dispatch.rs
  crates/slskr/src/legacy_route_dispatch_group_00.rs
  crates/slskr/src/legacy_route_dispatch_group_01.rs
  crates/slskr/src/legacy_route_dispatch_group_02.rs
  crates/slskr/src/legacy_route_dispatch_group_03.rs
  crates/slskr/src/legacy_route_dispatch_group_04.rs
  crates/slskr/src/legacy_route_dispatch_group_05.rs
  crates/slskr/src/legacy_route_dispatch_group_06.rs
  crates/slskr/src/legacy_route_dispatch_group_07.rs
  crates/slskr/src/legacy_route_dispatch_group_08.rs
  crates/slskr/src/legacy_route_dispatch_group_09.rs
  crates/slskr/src/legacy_route_dispatch_group_10.rs
  crates/slskr/src/controller_tests.rs
  crates/slskr/src/controller_tests/*.rs
  crates/slskr/src/destination_state.rs
  crates/slskr/src/database_maintenance.rs
  crates/slskr/src/event_store.rs
  crates/slskr/src/utils.rs
  crates/slskr/src/web_static.rs
  crates/slskr/src/podcore_controller.rs
  crates/slskr/src/mediacore_controller.rs
  crates/slskr/src/extended_controller.rs
  crates/slskr/src/file_transfer_runtime.rs
  crates/slskr/src/file_transfer_runtime_owners/*.rs
  crates/slskr/src/peer_transport.rs
  crates/slskr/src/peer_message_runtime.rs
  crates/slskr/src/http_connection.rs
  crates/slskr/src/transfer_controller.rs
  crates/slskr/src/transfer_batch_controller.rs
  crates/slskr/src/mesh_sync.rs
  crates/slskr/src/mesh_dht_runtime.rs
  crates/slskr/src/mesh_gateway_controller.rs
  crates/slskr/src/ranking_controller.rs
  crates/slskr/src/mesh_state.rs
  crates/slskr/src/bloom_filter.rs
  crates/slskr/src/multisource.rs
  crates/slskr/src/content_discovery.rs
  crates/slskr/src/realm_subject_index.rs
  crates/slskr/src/route_dispatch_group_0.rs
  crates/slskr/src/route_dispatch_group_2.rs
  crates/slskr/src/route_dispatch_group_2_session_search.rs
  crates/slskr/src/route_dispatch_group_2_downloads.rs
  crates/slskr/src/route_dispatch_group_2_transfer_status.rs
  crates/slskr/src/route_dispatch_group_2_transfer_files.rs
  crates/slskr/src/route_dispatch_group_2_search_rooms.rs
  crates/slskr/src/route_dispatch_group_4.rs
  crates/slskr/src/route_dispatch_group_4_collections.rs
  crates/slskr/src/route_dispatch_group_4_wishlist.rs
  crates/slskr/src/route_dispatch_group_4_contacts_sharegroups.rs
  crates/slskr/src/route_dispatch_group_4_notes_interests_grants.rs
  crates/slskr/src/route_dispatch_group_5.rs
  crates/slskr/src/route_dispatch_group_5_library_profile.rs
  crates/slskr/src/route_dispatch_group_5_conversations_jobs.rs
  crates/slskr/src/route_dispatch_group_5_configuration_bridge.rs
  crates/slskr/src/route_dispatch_group_5_mutations.rs
  crates/slskr/src/route_dispatch_group_6.rs
  crates/slskr/src/route_dispatch_group_6_admin_discovery.rs
  crates/slskr/src/route_dispatch_group_6_telemetry.rs
  crates/slskr/src/route_dispatch_group_6_media_jobs.rs
  crates/slskr/src/route_dispatch_group_6_security_shares.rs
  crates/slskr/src/route_dispatch_group_6_integrations.rs
  crates/slskr/src/route_dispatch_group_6_musicbrainz.rs
  crates/slskr/src/route_dispatch_group_7.rs
  crates/slskr/src/route_dispatch_group_7_media_profile_import.rs
  crates/slskr/src/route_dispatch_group_7_stream_services.rs
  crates/slskr/src/route_dispatch_group_7_podcore.rs
  crates/slskr/src/route_dispatch_group_7_network_admin.rs
  crates/slskr/src/route_request_entry.rs
  crates/slskr/src/focused_controller_tests.rs
  crates/slskr/src/focused_controller_tests/*.rs
  crates/slskr/src/contact_state.rs
  crates/slskr/src/collection_store.rs
  crates/slskr/src/controller_feature_state.rs
  crates/slskr/src/controller_options_state.rs
  crates/slskr/src/controller_options_projection.rs
  crates/slskr/src/controller_debug_view.rs
  crates/slskr/src/controller_yaml.rs
  crates/slskr/src/controller_release_check.rs
  crates/slskr/src/controller_capabilities.rs
  crates/slskr/src/controller_storage_preflight.rs
  crates/slskr/src/controller_storage.rs
  crates/slskr/src/incoming_share_store.rs
  crates/slskr/src/hash_backfill_state.rs
  crates/slskr/src/hash_backfill_controller.rs
  crates/slskr/src/hash_backfill_runtime.rs
  crates/slskr/src/integration_runtime_state.rs
  crates/slskr/src/integration_target.rs
  crates/slskr/src/lidarr_api.rs
  crates/slskr/src/lidarr_import.rs
  crates/slskr/src/lidarr_wishlist_sync.rs
  crates/slskr/src/versioned_get_contract.rs
  crates/slskr/src/versioned_relay_controller.rs
  crates/slskr/src/source_feed_ingest.rs
  crates/slskr/src/source_feed_preview_controller.rs
  crates/slskr/src/listening_party_stream_state.rs
  crates/slskr/src/library_store.rs
  crates/slskr/src/local_file_hash.rs
  crates/slskr/src/managed_tasks.rs
  crates/slskr/src/rate_limit.rs
  crates/slskr/src/oauth_state.rs
  crates/slskr/src/preview_stream_state.rs
  crates/slskr/src/preview_stream_controller.rs
  crates/slskr/src/relay_state.rs
  crates/slskr/src/runtime_compat_state.rs
  crates/slskr/src/security_state.rs
  crates/slskr/src/share_grant_store.rs
  crates/slskr/src/share_index_state.rs
  crates/slskr/src/share_index_runtime.rs
  crates/slskr/src/share_scanner.rs
  crates/slskr/src/source_discovery_state.rs
  crates/slskr/src/source_discovery_runtime.rs
  crates/slskr/src/songid_runtime.rs
  crates/slskr/src/vpn_runtime.rs
  crates/slskr/src/transfer_state_io.rs
  crates/slskr/src/transfer_durability.rs
  crates/slskr/src/transfer_queue.rs
  crates/slskr/src/transfer_recovery.rs
  crates/slskr/src/transfer_recovery_runtime.rs
  crates/slskr/src/transfer_entry.rs
  crates/slskr/src/session_state.rs
  crates/slskr/src/listener_state.rs
  crates/slskr/src/listener_runtime.rs
  crates/slskr/src/distributed_state.rs
  crates/slskr/src/distributed_runtime.rs
  crates/slskr/src/storage_directory_state.rs
  crates/slskr/src/wishlist_store.rs
  crates/slskr/src/wishlist_auto_download.rs
  crates/slskr/src/wishlist_csv_import.rs
  crates/slskr/src/browse_runtime.rs
  crates/slskr/src/browse_store.rs
  crates/slskr/src/browse_wire.rs
  crates/slskr/src/request_input.rs
  crates/slskr/src/search_store.rs
  crates/slskr/src/search_runtime.rs
  crates/slskr/src/message_store.rs
  crates/slskr/src/room_store.rs
  crates/slskr/src/user_store.rs
  crates/slskr/src/user_note_store.rs
)
http_source="crates/slskr/src/http_server.rs"
credential_source="crates/slskr/src/credential_store.rs"
config_sources=(crates/slskr/src/config.rs crates/slskr/src/config_parts/*.rs)
config_tests_sources=(crates/slskr/src/config_tests.rs crates/slskr/src/config_tests/*.rs)
client_social_source="crates/slskr-client/src/social.rs"
client_capability_source="crates/slskr-client/src/capabilities.rs"
client_peer_cache_source="crates/slskr-client/src/peer_cache.rs"
client_distributed_tree_source="crates/slskr-client/src/distributed_tree.rs"
client_filter_source="crates/slskr-client/src/filters.rs"
status=0

for anchor in \
  'ensure_private_state_dir(&config.state_dir)' \
  'state_directory_is_private_and_rejects_symlinks' \
  'wait_for_reconnect_or_command' \
  'reconnect_backoff_is_interrupted_by_session_commands' \
  'controller_storage_directory_json_unix' \
  'storage directory confined open failed' \
  'scoped_storage_listing_rejects_symlinked_parent' \
  'open_shared_local_file_unix' \
  'open_shared_local_file(state, &path)' \
  'shared directory confined open failed' \
  'shared_file_confined_open_rejects_symlinked_parent' \
  'read_file_chunk(file, offset, length, indexed_size)' \
  'mesh_sync_chunk_reads_remain_confined_to_share_roots' \
  'scan_share_root_unix' \
  'share_scan_does_not_follow_symlinked_directory' \
  'MAX_SHARE_SCAN_ENTRIES' \
  'share_scan_bounds_aggregate_directory_entries' \
  'MAX_SHARE_SCAN_PENDING_DIRECTORIES' \
  'share_scan_bounds_pending_directory_descriptors' \
  'MAX_SHARE_SCAN_TASKS' \
  'share_rebuild_routes_reject_concurrent_scans' \
  'build_share_index_with_cancellation(&config, cancellation)' \
  'SHARE_SCAN_CANCELLED_ERROR' \
  'share_rebuild_errors_redact_internal_details' \
  'share_snapshot_errors_redact_internal_details' \
  'transfer_storage_errors_redact_internal_details' \
  'transfer_json_redacts_local_storage_details' \
  'listener_errors_redact_internal_details' \
  'session_errors_redact_internal_details' \
  'browse_errors_redact_internal_details' \
  'browse_failure_events_redact_internal_details' \
  'external_visualizer_launch_errors_redact_command_details' \
  'options_config_location_is_the_confined_compatibility_file' \
  'toml_config_sanitizes_secrets_and_storage_paths' \
  'lidarr_projections_redact_endpoint_and_errors' \
  'bridge_projections_redact_internal_endpoint' \
  'spotify_diagnostic_projections_redact_redirect_uri' \
  'share_tokens_use_headers_and_content_bound_stream_tickets' \
  'share_grants_require_live_collections_and_are_revoked_on_delete' \
  'share_access_tokens_persist_only_digests_and_rehydrate' \
  'share_access_token_issue_rolls_back_when_persistence_fails' \
  'wishlist_ignored_folders_persist_and_suppress_existing_and_future_results' \
  'wishlist_ignored_folder_mutations_roll_back_on_persistence_failure' \
  'MAX_WISHLIST_FILTER_BYTES' \
  'MAX_WISHLIST_RESULTS' \
  'MAX_WISHLIST_DOWNLOADS' \
  'add_item_with_settings' \
  'completed_transfer_cleanup_rolls_back_on_persistence_failure' \
  'transfer_cancel_rolls_back_on_persistence_failure' \
  '.redirect(reqwest::redirect::Policy::none())' \
  'MAX_INTEGRATION_RESPONSE_BYTES' \
  'integration_json_reader_rejects_declared_oversized_response' \
  'integration_json_reader_rejects_chunked_oversized_response' \
  'integration_ssrf_filter_blocks_special_use_ip_ranges' \
  'is_blocked_outbound_ipv4' \
  'nat64_embedded_ipv4' \
  'MAX_SEARCH_RESULTS_PER_SEARCH' \
  'search_store_caps_results_from_peer_responses' \
  'libc::O_NOFOLLOW' \
  '.take(MAX_TRANSFER_STATE_BYTES + 1)' \
  'transfer_state_loader_rejects_fifo_without_blocking' \
  'transfer event path must be a regular file' \
  'open_transfer_event_file' \
  'transfer_event_open_rejects_fifo_without_blocking' \
  'write_file_atomic_with_temp_path' \
  'state_file_io_rejects_symlinks_without_touching_targets' \
  'MAX_ROOM_MESSAGES_PER_ROOM' \
  'room_message_history_evicts_oldest_entries_at_limit' \
  'MAX_ROOM_RECORDS' \
  'MAX_ROOM_MEMBERS_PER_ROOM' \
  'room_store_rejects_new_records_at_limit_but_updates_existing_rooms' \
  'room_store_rejects_new_members_at_limit_but_accepts_duplicates' \
  'MAX_USER_RECORDS' \
  'user_store_rejects_new_records_at_limit_but_updates_existing_users' \
  'MAX_BROWSE_RECORDS' \
  'MAX_BROWSE_ENTRIES_PER_USER' \
  'MAX_BROWSE_WIRE_SECTIONS_PER_RESPONSE' \
  'MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE' \
  'MAX_BROWSE_WIRE_FILES_PER_RESPONSE' \
  'shared_file_list_payload_rejects_excessive_wire_records_without_entries' \
  'shared_file_list_payload_rejects_excessive_sections_without_files' \
  'folder_browse_parsers_reject_excessive_wire_records' \
  'MAX_BLOOM_FILTER_BYTES' \
  'filter_rejects_unbounded_precision_requests' \
  'validate_file_size_and_chunk_size' \
  'versioned_swarm_rejects_oversized_transfer_limits_before_discovery' \
  'versioned_swarm_rejects_oversized_source_batches_before_deserialization' \
  'source count exceeds the 16 source limit' \
  'discover_mesh_range_sources' \
  'sources.truncate(multisource::MAX_SOURCES)' \
  'MAX_HASH_MERGE_ENTRIES' \
  'MAX_MESH_MERGE_ENTRIES' \
  'MAX_SHADOW_MERGE_RECORDS' \
  'MAX_INDEXES' \
  'json_array_exceeds_limit' \
  'merge_routes_reject_oversized_arrays_before_store_deserialization' \
  'expectedHash is required for verified swarm execution' \
  'versioned_swarm_requires_expected_hash_before_queueing' \
  'multisource_versioned_download_response' \
  'versioned_download_range_sources_use_verified_executor' \
  'browse_response_exceeds_wire_limits' \
  'browse_response_rejects_oversized_wire_batches_before_store_mutation' \
  'search_response_exceeds_wire_limits' \
  'search_response_rejects_oversized_wire_batches_before_store_mutation' \
  'collection_reorder_exceeds_wire_limits' \
  'collection_reorder_rejects_oversized_wire_batches_before_store_mutation' \
  'musicbrainz_rejects_oversized_json_batches_before_state_mutation' \
  'MAX_TRANSFER_REQUEST_FILES' \
  'controller_file_array_exceeds_wire_limits' \
  'transfer_rejects_oversized_file_batches_before_queue_mutation' \
  'MAX_CAPABILITY_NEGOTIATION_ITEMS' \
  'json_array_field_exceeds_limit' \
  'conversation_batch_exceeds_wire_limits' \
  'wishlist_bulk_filter_exceeds_wire_limits' \
  'string_array_routes_reject_oversized_wire_batches_before_mutation' \
  'MAX_MEDIACORE_BATCH_ITEMS' \
  'mediacore_rejects_oversized_wire_batches_before_work' \
  'MAX_MEDIACORE_PORTABILITY_ENTRIES' \
  'MAX_QUARANTINE_JURY_ITEMS' \
  'MAX_EXTENDED_DOWNLOAD_ITEMS' \
  'MAX_RANKING_BATCH_ITEMS' \
  'MAX_LISTENING_PARTY_TAGS' \
  'remaining_controller_array_routes_reject_oversized_wire_batches_before_work' \
  'MAX_ROUTING_TARGET_PEERS' \
  'MAX_RADAR_MUTED_RELEASE_GROUPS' \
  'MAX_POD_DISCOVERY_TAGS' \
  'legacy-route-dispatch' \
  'legacy dispatcher merge-array parity' \
  'legacy merge paths use bounded route dispatcher' \
  'MAX_CSV_IMPORT_ROWS' \
  'MAX_SOURCE_PREVIEW_ROWS' \
  'parse_simple_wishlist_import_rows' \
  'parse_simple_source_preview_items' \
  'wishlist_csv_import_rejects_oversized_row_batches' \
  'MAX_ENTRIES_PER_INDEX' \
  'MAX_ALIASES_PER_ENTRY' \
  'MAX_EVIDENCE_LINKS_PER_ENTRY' \
  'MAX_EXTERNAL_IDS_PER_ENTRY' \
  'MAX_WORK_REF_METADATA_FIELDS' \
  'merge_rejects_oversized_nested_index_collections_before_hashing' \
  'issue_ids must contain 1 to 25 values' \
  'browse_store_bounds_records_and_entries_but_updates_existing_users' \
  'MAX_MESSAGE_RECORDS' \
  'message_store_evicts_oldest_records_at_limit' \
  'MAX_OAUTH_STATES' \
  'OAuthStateStore' \
  'spotify_oauth_state_persists_rehydrates_and_consumes' \
  'spotify_oauth_state_is_not_consumed_when_persistence_delete_fails' \
  'oauth_state_consumption_is_ordered_and_keeps_reads_available' \
  'MAX_PREVIEW_STREAM_TICKETS' \
  'MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS' \
  'is_private_message_auto_response_candidate' \
  'private_message_auto_response_classifier_and_cooldown_are_bounded' \
  'transient_credential_stores_refuse_bursts_at_live_capacity' \
  'MAX_CONTACT_RECORDS' \
  'contacts_are_bounded_deduplicated_and_report_discovery_truthfully' \
  'MAX_SHARE_GROUPS' \
  'MAX_SHARE_GROUP_MEMBERS' \
  'share_groups_bound_groups_and_case_insensitive_members' \
  'MAX_COLLECTIONS' \
  'MAX_COLLECTION_ITEMS' \
  'collections_bound_nested_state_and_allocate_unique_item_ids' \
  'MAX_WISHLIST_ITEMS' \
  'wishlist_bounds_items_and_allocates_unique_ids' \
  'MAX_LIBRARY_HEALTH_SCANS' \
  'library_health_scans_are_bounded_snapshots_with_unique_ids' \
  'MAX_SONGID_RUNS' \
  'songid_runs_are_bounded_snapshots_with_real_lookup' \
  'MAX_USER_NOTES' \
  'MAX_INTERESTS_PER_KIND' \
  'notes_and_interests_bound_growth_and_ids' \
  'MAX_NOW_PLAYING_RECORDS' \
  'MAX_SECURITY_BANS' \
  'jwt-revocations.json' \
  'JWT revocation state exceeds its record or identifier limits' \
  'RevokedJwtStore' \
  'LoginAttemptStore' \
  'revoked_jwt_store_persists_and_reloads_across_restart' \
  'revoked_jwt_store_prunes_expired_entries_on_reload' \
  'versioned_session_bounds_failed_login_attempts' \
  'now_playing_and_security_state_bound_remote_keys' \
  'MAX_SOURCE_FEED_IMPORT_HISTORY' \
  'MAX_SOURCE_FEED_HISTORY_SUGGESTIONS' \
  'MAX_SOURCE_FEED_HISTORY_SKIPPED_ROWS' \
  'MAX_SOURCE_FEED_PREVIEW_LENGTH' \
  'SourceFeedImportHistoryStore' \
  'LidarrSyncRuntimeState' \
  'SpotifyConnectionStore' \
  'ProtectedSpotifyConnection' \
  'spotify_authorization_exchanges_profiles_persists_and_disconnects' \
  'spotify_connection_status_and_disconnect' \
  'spotify_oauth_state_persists_rehydrates_and_consumes' \
  'controller_api_differential_lidarr_and_source_feed_contracts' \
  'controller_api_differential_source_feed_open_cases' \
  'MAX_SHARE_GRANTS' \
  'share_grants_bound_and_deduplicate_collection_users' \
  'MAX_LIBRARY_ITEMS' \
  'library_items_bound_growth_and_checked_ids' \
  'MAX_DESTINATIONS' \
  'destinations_bound_deduplicate_and_select_one_default' \
  'MAX_SEARCH_RECORDS' \
  'searches_bound_active_records_and_avoid_identity_collisions' \
  'sync_invalid_messages' \
  'sync_violation_state_enforces_message_and_entry_limits_separately' \
  'create_auto_retry_plan' \
  'create_rescue_plan' \
  'transfer_ids_and_tokens_wrap_without_collisions' \
  'finite transfer history must leave an available u64 id' \
  'bounded_store_ids_wrap_without_collisions' \
  'bounded event history must leave an available u64 id' \
  'bounded_content_store_ids_wrap_without_collisions' \
  'bounded user-note store must leave an available u64 id' \
  'bounded library scan store must leave an available u64 id' \
  'collection_and_wishlist_ids_wrap_without_collisions' \
  'bounded collection items must leave an available u64 id' \
  'bounded wishlist store must leave an available u64 id' \
  'bounded SongID run history must leave an available u64 id' \
  'open_shared_local_file(state, &shared_file.local_path)' \
  'options.custom_flags(libc::O_NOFOLLOW)' \
  'open_download_file_for_read(&downloads_dir, &path)' \
  'download_confined_open_rejects_symlinked_parent' \
  'download directory confined open failed' \
  'scoped_storage_confined_delete_rejects_symlinked_parent' \
  'storage parent confined open failed' \
  'remove_directory_contents_unix' \
  'read_bounded_web_static_file_under_root(&root, &file)' \
  'static directory confined open failed' \
  '.take(MAX_WEB_STATIC_BYTES + 1)' \
  'browse_indirect_tokens_wrap_without_aliasing_pending_records' \
  'bounded browse store must leave an available u32 token' \
  'MAX_INCOMING_CONNECTION_TASKS' \
  'MAX_HTTP_CONNECTION_TASKS' \
  'time::timeout(http_server::RESPONSE_WRITE_TIMEOUT' \
  'state.config.peer_response_timeout' \
  'state.incoming_connections' \
  'spawn_bounded_http_connection_task' \
  'spawn_bounded_http_task(&unix_connections' \
  'spawn_bounded_http_task(&tls_connections' \
  'http_listener_connection_tasks_share_capacity_and_join_with_managed_shutdown' \
  'runtime_compatibility_mutation_releases_store_guards_during_sqlite_io' \
  'runtime_compatibility_transient_latches_stay_process_local' \
  'lidarr_manual_import_releases_store_guards_during_sqlite_io' \
  'share_grant_create_releases_store_readers_during_sqlite_io' \
  'collection_delete_releases_store_guards_during_sqlite_io' \
  'route_dispatch::share_grant_store_matches(&grants, &mutated)' \
  'route_dispatch::share_grant_store_matches(&grants, &mutated_grants)' \
  'route_dispatch::rollback_library_runtime_if_unchanged(' \
  'upsert_library_item_and_runtime_compat_state' \
  'Never let an unrelated durable compatibility write turn them'; do
  if ! rg -n --fixed-strings -- "$anchor" "${source_files[@]}" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'DEFAULT_MAX_DISTRIBUTED_CHILDREN' \
  'DistributedChildCapacityFull' \
  'distributed_tree_rejects_new_children_at_limit_but_allows_replacement'; do
  if ! rg -n --fixed-strings -- "$anchor" "$client_distributed_tree_source" crates/slskr-client/src/error.rs crates/slskr-client/tests/distributed_tree.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing distributed tree anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'DEFAULT_MAX_PEER_CONNECTIONS' \
  'PeerConnectionCacheFull' \
  'cache_rejects_new_peers_at_limit_but_allows_replacement'; do
  if ! rg -n --fixed-strings -- "$anchor" "$client_peer_cache_source" crates/slskr-client/src/error.rs crates/slskr-client/tests/peer_cache.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing peer cache anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'MAX_PEER_CAPABILITY_RECORDS' \
  'CapabilityError::RegistryFull' \
  'registry_prunes_expired_records_and_rejects_new_peers_at_limit'; do
  if ! rg -n --fixed-strings -- "$anchor" "$client_capability_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing client capability anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'DEFAULT_MAX_USER_WATCH_RECORDS' \
  'user_watch_state_rejects_new_users_at_limit_but_updates_existing_users' \
  'DEFAULT_MAX_JOINED_ROOMS' \
  'room_state_rejects_new_rooms_at_limit_but_keeps_existing_room_messages' \
  'with_max_records' \
  'MAX_STORED_ROOM_MESSAGES' \
  'MAX_STORED_PRIVATE_MESSAGES' \
  'retain_newest(&mut self.messages'; do
  if ! rg -n --fixed-strings -- "$anchor" "$client_social_source" crates/slskr-client/tests/phase7.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing client social anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'MAX_SEARCH_RESPONSES_PER_TOKEN' \
  'MAX_SEARCH_RESULT_FILES_PER_TOKEN' \
  'search_results_bound_responses_and_files_per_token'; do
  if ! rg -n --fixed-strings -- "$anchor" crates/slskr-client/src/search.rs crates/slskr-client/tests/search.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing client search anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'MAX_EXCLUDED_SEARCH_PHRASES' \
  'MAX_EXCLUDED_SEARCH_PHRASE_BYTES' \
  'MAX_FILTERED_SEARCH_QUERY_BYTES' \
  'excluded_phrase_filter_is_literal_and_bounds_remote_inputs'; do
  if ! rg -n --fixed-strings -- "$anchor" "$client_filter_source" crates/slskr-client/tests/phase7.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing search-filter anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'REQUEST_READ_TIMEOUT' \
  'RESPONSE_WRITE_TIMEOUT' \
  'test_request_deadline_is_not_reset_by_partial_progress' \
  'test_response_write_deadline_releases_blocked_writer' \
  'test_http11_requires_one_nonempty_host_header' \
  'test_duplicate_authentication_headers_are_rejected' \
  'test_repeated_forwarding_headers_are_combined_in_wire_order'; do
  if ! rg -n --fixed-strings -- "$anchor" "$http_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing HTTP anchor %s\n' "$anchor" >&2
    status=1
  fi
done


for anchor in \
  'WEBSOCKET_WRITE_TIMEOUT' \
  'websocket_client_frame_rejects_non_canonical_lengths' \
  'websocket_client_frame_rejects_reserved_length_high_bit' \
  'websocket_write_deadline_releases_blocked_writer'; do
  if ! rg -n --fixed-strings -- "$anchor" crates/slskr/src/events_ws.rs >/dev/null; then
    printf 'runtime boundary hardening check failed: missing WebSocket anchor %s\n' "$anchor" >&2
    status=1
  fi
done

if ! rg -n -U -P \
  'http_server::write_http_response(?:_with_body)?\([\s\S]{0,400}?\)\s*\.await\?;' \
  "${source_files[@]}" >/dev/null; then
  printf 'runtime boundary hardening check failed: API response write failures must terminate the connection\n' >&2
  status=1
fi

for anchor in \
  'MAX_CREDENTIAL_FILE_BYTES' \
  'credential_file_write_rejects_symlink_without_touching_target' \
  'credential_file_read_rejects_symlink_without_reading_target' \
  'credential_file_read_rejects_oversized_input' \
  'credential_parent_validation_does_not_mutate_existing_permissions' \
  'credential_parent_validation_rejects_shared_writable_directory'; do
  if ! rg -n --fixed-strings -- "$anchor" "$credential_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing credential anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'file.take(MAX_CONFIG_FILE_BYTES + 1)' \
  'HTTP API token must not be empty or whitespace-only'; do
  if ! rg -n --fixed-strings -- "$anchor" "${config_sources[@]}" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing config reader anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'api_token_rejects_blank_env_and_file_values' \
  'config_file_reader_rejects_symlinks' \
  'config_file_reader_rejects_oversized_files'; do
  if ! rg -n --fixed-strings -- "$anchor" "${config_tests_sources[@]}" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing config reader anchor %s\n' "$anchor" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

# Gateway children belong to local cancellation owners and the daemon join set.
if rg -n 'tokio::(task::)?spawn\(' \
  crates/slskr/src/private_gateway_owners/gateway_models.rs \
  crates/slskr/src/private_gateway_owners/gateway_services.rs \
  crates/slskr/src/private_gateway_owners/gateway_transport.rs \
  crates/slskr/src/private_gateway_owners/gateway_udp_runtime.rs \
  crates/slskr/src/private_gateway_owners/shared_quic_runtime.rs \
  crates/slskr/src/private_gateway_owners/quic_proxy.rs; then
  printf 'runtime boundary hardening failed: detached gateway task\n' >&2
  exit 1
fi

# CLI live-soak workers remain bounded and owned by their enclosing scopes.
if rg -n 'tokio::(task::)?spawn\(' \
  crates/slskr/src/cli_smoke_soak_owners/live_peer_runtime.rs \
  crates/slskr/src/cli_smoke_soak_owners/live_server_runtime.rs \
  crates/slskr/src/cli_smoke_soak_owners/live_soak_entry.rs; then
  printf 'runtime boundary hardening failed: detached live-soak task\n' >&2
  exit 1
fi

# Short-lived CLI fixture and accept tasks have cancellation owners too.
if rg -n 'tokio::(task::)?spawn\(' \
  crates/slskr/src/cli_smoke_soak_owners/fixture_transfers.rs \
  crates/slskr/src/cli_smoke_soak_owners/peer_probe_operations.rs \
  crates/slskr/src/cli_smoke_soak_owners/peer_smoke_scenarios.rs \
  crates/slskr/src/cli_smoke_soak_owners/server_smoke_scenarios.rs \
  crates/slskr/src/cli_smoke_soak_owners/transfer_smoke_scenarios.rs; then
  printf 'runtime boundary hardening failed: detached CLI probe task\n' >&2
  exit 1
fi

# Administrative jobs share daemon shutdown ownership; forwarding has one
# explicitly owned listener task and a bounded child join set.
if rg -n 'tokio::(task::)?spawn\(' \
  crates/slskr/src/route_dispatch.rs \
  crates/slskr/src/route_dispatch_group_1_events.rs \
  crates/slskr/src/route_dispatch_group_3_admin_controls.rs \
  crates/slskr/src/transfer_completion.rs \
  crates/slskr/src/virtual_soulfind_v2_controller.rs \
  crates/slskr/src/webhooks.rs \
  crates/slskr/src/scripts.rs; then
  printf 'runtime boundary hardening failed: detached administrative worker\n' >&2
  exit 1
fi
python3 - <<'PYFORWARD'
from pathlib import Path
source = Path('crates/slskr/src/port_forwarding.rs').read_text()
if source.count('tokio::spawn(') != 1 or 'Some(ListenerTask(task))' not in source:
    raise SystemExit('runtime boundary hardening failed: forwarding listener ownership changed')
for anchor in ('connections.spawn(', 'connections.shutdown().await', 'impl Drop for ListenerTask', 'impl Drop for ActiveConnection'):
    if anchor not in source:
        raise SystemExit(f'runtime boundary hardening failed: missing forwarding owner {anchor}')
PYFORWARD

# WebSocket readers stay inside their connection future, including cancellation.
python3 - <<'PYWS'
from pathlib import Path
for name in ('events_ws.rs', 'signalr_ws.rs', 'relay_ws.rs'):
    path = Path('crates/slskr/src') / name
    production = path.read_text().split('#[cfg(test)]', 1)[0]
    if 'tokio::spawn(' in production or 'tokio::task::spawn(' in production:
        raise SystemExit(f'runtime boundary hardening failed: detached WebSocket reader in {path}')
    if 'tokio::pin!(reader_task,' not in production:
        raise SystemExit(f'runtime boundary hardening failed: missing scoped reader in {path}')
PYWS

python3 - <<'PYFTP'
from pathlib import Path
source = Path('crates/slskr/src/ftp_upload_queue.rs').read_text()
for anchor in ('MAX_ADMITTED_UPLOADS: usize = 64', 'MAX_ACTIVE_UPLOADS: usize = 4',
               'tasks.try_spawn(', 'impl Drop for FtpUploadQueue'):
    if anchor not in source:
        raise SystemExit(f'runtime boundary hardening failed: missing FTP admission {anchor}')
state = Path('crates/slskr/src/app_state.rs').read_text().split('async fn shutdown_managed_tasks', 1)[1]
if state.index('self.ftp_uploads.close()') > state.index('self.managed_background_tasks.shutdown()'):
    raise SystemExit('runtime boundary hardening failed: FTP admission must close before joining workers')
PYFTP

python3 - <<'PYRELAY'
from pathlib import Path
cleanup = Path('crates/slskr/src/relay_connection_cleanup.rs').read_text()
for anchor in ('MAX_PENDING_CLEANUPS: usize = 256', 'reserve_owned()',
               'permit.send(', 'impl Drop for ConnectionLease', 'Arc::downgrade(state)',
               'managed_background_tasks.try_spawn('):
    if anchor not in cleanup:
        raise SystemExit(f'runtime boundary hardening failed: missing relay cleanup owner {anchor}')
if 'tokio::spawn(' in cleanup or 'tokio::task::spawn(' in cleanup:
    raise SystemExit('runtime boundary hardening failed: detached relay cleanup worker')
state = Path('crates/slskr/src/app_state.rs').read_text().split('async fn shutdown_managed_tasks', 1)[1]
if state.index('self.relay_cleanup.close()') > state.index('self.managed_background_tasks.shutdown()'):
    raise SystemExit('runtime boundary hardening failed: relay cleanup admission must close before shutdown')
if 'protocol.shutdown_connections()' not in state:
    raise SystemExit('runtime boundary hardening failed: relay registrations must clear after shutdown')
PYRELAY

python3 - <<'PYWEBHOOK'
from pathlib import Path
state = Path('crates/slskr/src/app_state.rs').read_text().split('async fn shutdown_managed_tasks', 1)[1]
if state.index('fail_unconfirmed_webhook_logs(') < state.index('self.managed_background_tasks.shutdown()'):
    raise SystemExit('runtime boundary hardening failed: webhook outcomes must reconcile after joining workers')
startup = Path('crates/slskr/src/daemon_serve.rs').read_text()
if 'fail_unconfirmed_webhook_logs(' not in startup:
    raise SystemExit('runtime boundary hardening failed: missing interrupted webhook recovery')
schema = Path('crates/slskr/src/persistence.rs').read_text()
if 'idx_webhook_logs_queued' not in schema:
    raise SystemExit('runtime boundary hardening failed: missing queued webhook reconciliation index')
PYWEBHOOK

# One-shot QUIC operations finish cleanup in their caller's scope.
python3 - <<'PYCODE'
from pathlib import Path
for name in ('quic_control.rs', 'quic_data.rs', 'shared_udp.rs', 'shared_quic_server.rs'):
    path = Path('crates/slskr-client/src') / name
    production = path.read_text().split('#[cfg(test)]', 1)[0]
    if 'tokio::spawn(' in production or 'tokio::task::spawn(' in production:
        raise SystemExit(f'runtime boundary hardening failed: detached QUIC worker in {path}')
route = Path('crates/slskr/src/private_gateway_owners/shared_quic_runtime.rs').read_text()
if 'first_alpn(' in route:
    raise SystemExit('runtime boundary hardening failed: native QUIC must negotiate ALPN after reassembly')
PYCODE

printf 'runtime boundary hardening check passed\n'
