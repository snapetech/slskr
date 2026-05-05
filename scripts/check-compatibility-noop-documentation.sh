#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-010 BUG-011; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'compatibility no-op documentation check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

for route in '/api/options' '/api/options/yaml' '/api/options/yaml/location' 'logs/cache/bridge/config/bans/share-grant token' 'MusicBrainz subscription'; do
  if ! rg -n -F "$route" "$ledger" docs/security-bug-burndown.md >/dev/null; then
    printf 'compatibility no-op documentation check failed: expected inventory token missing: %s\n' "$route" >&2
    status=1
  fi
done

for expected in 'compatibility endpoint is read-only' 'not active in this runtime' 'acknowledgement|acknowledged|non-persisted'; do
  if ! rg -n "$expected" crates/slskr/src/main.rs docs/security-bug-burndown.md "$ledger" >/dev/null; then
    printf 'compatibility no-op documentation check failed: expected implementation/docs token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'compatibility no-op documentation check passed\n'
