#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source_file="crates/slskr/src/main.rs"
http_source="crates/slskr/src/http_server.rs"
credential_source="crates/slskr/src/credential_store.rs"
status=0

for anchor in \
  'ensure_private_state_dir(&config.state_dir)' \
  'state_directory_is_private_and_rejects_symlinks' \
  '.redirect(reqwest::redirect::Policy::none())' \
  'MAX_INTEGRATION_RESPONSE_BYTES' \
  'integration_json_reader_rejects_declared_oversized_response' \
  'integration_json_reader_rejects_chunked_oversized_response' \
  'integration_ssrf_filter_blocks_special_use_ipv6_ranges' \
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
