#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

require_ledger_item() {
  local id="$1"
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'websocket auth coverage check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
}

require_ledger_item BUG-003
require_ledger_item BUG-004

if ! rg -n 'GET", "/api/events/ws"|websocket_path == "/api/events/ws"' crates/slskr/src/main.rs >/dev/null; then
  printf 'websocket auth coverage check failed: server event WebSocket route was not found\n' >&2
  status=1
fi

if rg -n 'new WebSocket\(this\.url\)' client-ts/src/websocket-client.ts >/dev/null; then
  printf 'client-ts browser WebSocket auth remains accepted-open in BUG-003\n'
elif ! rg -n 'Sec-WebSocket-Protocol|ticket|events/ws.*token|event.*ticket' client-ts/src web/src crates/slskr/src >/dev/null; then
  printf 'websocket auth coverage check failed: client-ts gap changed without visible replacement auth path\n' >&2
  status=1
fi

if rg -n 'new WebSocket\(eventFeedUrl\(\)\)' web/src/lib/hubFactory.js >/dev/null; then
  printf 'React web event-feed auth remains accepted-open in BUG-004\n'
elif ! rg -n 'Sec-WebSocket-Protocol|ticket|events/ws.*token|event.*ticket' web/src crates/slskr/src >/dev/null; then
  printf 'websocket auth coverage check failed: React event-feed gap changed without visible replacement auth path\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'websocket auth coverage check passed\n'
