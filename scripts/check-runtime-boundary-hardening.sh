#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source_file="crates/slskr/src/main.rs"
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

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'runtime boundary hardening check passed\n'
