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
  printf 'websocket auth coverage check failed: client-ts event feed still opens without auth subprotocols\n' >&2
  status=1
elif ! rg -n 'websocketAuthProtocols|slskr\.api-token\.' client-ts/src/websocket-client.ts >/dev/null; then
  printf 'websocket auth coverage check failed: client-ts event feed lacks browser-safe auth subprotocol construction\n' >&2
  status=1
fi

if rg -n 'new WebSocket\(eventFeedUrl\(\)\)' web/src/lib/hubFactory.js >/dev/null; then
  printf 'websocket auth coverage check failed: React web event feed still opens without auth subprotocols\n' >&2
  status=1
elif ! rg -n 'eventFeedProtocols|slskr\.api-token\.' web/src/lib/hubFactory.js web/src/lib/hubFactory.test.js >/dev/null; then
  printf 'websocket auth coverage check failed: React event feed lacks browser-safe auth subprotocol coverage\n' >&2
  status=1
fi

if ! rg -n 'sec_websocket_protocol|websocket_protocol_authorization|Sec-WebSocket-Protocol' crates/slskr/src/main.rs crates/slskr/src/http_server.rs crates/slskr/src/events_ws.rs >/dev/null; then
  printf 'websocket auth coverage check failed: server does not parse and echo websocket auth subprotocols\n' >&2
  status=1
fi

for anchor in \
  'MAX_WEBSOCKET_CONNECTIONS' \
  'state.websocket_connections' \
  'websocket_connection_pool_reserves_http_capacity'; do
  if ! rg -n --fixed-strings -- "$anchor" crates/slskr/src/main.rs >/dev/null; then
    printf 'websocket auth coverage check failed: missing WebSocket admission anchor %s\n' "$anchor" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'websocket auth coverage check passed\n'
