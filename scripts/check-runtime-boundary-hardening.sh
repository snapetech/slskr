#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source_file="crates/slskr/src/main.rs"
http_source="crates/slskr/src/http_server.rs"
credential_source="crates/slskr/src/credential_store.rs"
client_social_source="crates/slskr-client/src/social.rs"
client_capability_source="crates/slskr-client/src/capabilities.rs"
client_peer_cache_source="crates/slskr-client/src/peer_cache.rs"
client_distributed_tree_source="crates/slskr-client/src/distributed_tree.rs"
status=0

for anchor in \
  'ensure_private_state_dir(&config.state_dir)' \
  'state_directory_is_private_and_rejects_symlinks' \
  'slskd_storage_directory_json_unix' \
  'storage directory confined open failed' \
  'scoped_storage_listing_rejects_symlinked_parent' \
  '.redirect(reqwest::redirect::Policy::none())' \
  'MAX_INTEGRATION_RESPONSE_BYTES' \
  'integration_json_reader_rejects_declared_oversized_response' \
  'integration_json_reader_rejects_chunked_oversized_response' \
  'integration_ssrf_filter_blocks_special_use_ipv6_ranges' \
  'MAX_SEARCH_RESULTS_PER_SEARCH' \
  'search_store_caps_results_from_peer_responses' \
  'libc::O_NOFOLLOW' \
  '.take(MAX_TRANSFER_STATE_BYTES + 1)' \
  'transfer event path must be a regular file' \
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
  'browse_store_bounds_records_and_entries_but_updates_existing_users' \
  'MAX_MESSAGE_RECORDS' \
  'message_store_evicts_oldest_records_at_limit' \
  'MAX_OAUTH_STATES' \
  'MAX_PREVIEW_STREAM_TICKETS' \
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
  'now_playing_and_security_state_bound_remote_keys' \
  'MAX_SHARE_GRANTS' \
  'share_grants_bound_and_deduplicate_collection_users' \
  'MAX_LIBRARY_ITEMS' \
  'library_items_bound_growth_and_checked_ids' \
  'MAX_DESTINATIONS' \
  'destinations_bound_deduplicate_and_select_one_default' \
  'MAX_SEARCH_RECORDS' \
  'searches_bound_active_records_and_avoid_identity_collisions' \
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
  'open_download_file(&download_root(&state.config.state_dir), &path)' \
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
  'time::timeout(http_server::RESPONSE_WRITE_TIMEOUT' \
  'state.config.peer_response_timeout' \
  'state.incoming_connections'; do
  if ! rg -n --fixed-strings -- "$anchor" "$source_file" >/dev/null; then
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

if ! rg -n --fixed-strings -- \
  'http_server::write_http_response(&mut writer, &response, keep_alive, &extra).await?;' \
  "$source_file" >/dev/null; then
  printf 'runtime boundary hardening check failed: API response write failures must terminate the connection\n' >&2
  status=1
fi

for anchor in \
  'MAX_CREDENTIAL_FILE_BYTES' \
  'credential_file_write_rejects_symlink_without_touching_target' \
  'credential_file_read_rejects_oversized_input' \
  'credential_parent_validation_does_not_mutate_existing_permissions' \
  'credential_parent_validation_rejects_shared_writable_directory'; do
  if ! rg -n --fixed-strings -- "$anchor" "$credential_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing credential anchor %s\n' "$anchor" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'runtime boundary hardening check passed\n'
