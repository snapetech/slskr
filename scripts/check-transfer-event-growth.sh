#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-024 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'transfer event growth check failed: BUG-024 must stay verified in council ledger\n' >&2
  status=1
fi

for expected in \
  'MAX_TRANSFER_EVENTS_BYTES' \
  'rotate_transfer_events_if_needed' \
  'transfer_event_append_rotates_oversized_event_file'
do
  if ! rg -n -F "$expected" crates/slskr/src/main.rs >/dev/null; then
    printf 'transfer event growth check failed: expected rotation token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'transfer event growth check passed\n'
