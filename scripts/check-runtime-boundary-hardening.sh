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
  'MAX_INCOMING_CONNECTION_TASKS' \
  'state.config.peer_response_timeout' \
  'state.incoming_connections'; do
  if ! rg -n --fixed-strings -- "$anchor" "$source_file" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'REQUEST_READ_TIMEOUT' \
  'test_request_deadline_is_not_reset_by_partial_progress'; do
  if ! rg -n --fixed-strings -- "$anchor" "$http_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing HTTP anchor %s\n' "$anchor" >&2
    status=1
  fi
done

for anchor in \
  'MAX_CREDENTIAL_FILE_BYTES' \
  'credential_file_write_rejects_symlink_without_touching_target' \
  'credential_file_read_rejects_oversized_input'; do
  if ! rg -n --fixed-strings -- "$anchor" "$credential_source" >/dev/null; then
    printf 'runtime boundary hardening check failed: missing credential anchor %s\n' "$anchor" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'runtime boundary hardening check passed\n'
