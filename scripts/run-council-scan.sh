#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

scan_rg() {
  local title="$1"
  shift
  local tmp
  tmp="$(mktemp)"
  rg -n --glob '!target/**' --glob '!web/node_modules/**' --glob '!dashboard/node_modules/**' \
    --glob '!client-ts/node_modules/**' --glob '!**/dist/**' --glob '!**/package-lock.json' \
    "$@" >"$tmp" || true
  local count
  count="$(wc -l <"$tmp" | tr -d ' ')"
  printf '| %s | %s |\n' "$title" "$count"
  rm -f "$tmp"
}

printf '# Council Scan Candidate Counts\n\n'
printf 'Generated from local source patterns. Counts are candidate lines, not confirmed bugs.\n\n'
printf '| Candidate Class | Count |\n'
printf '| --- | ---: |\n'
scan_rg 'Constructor/mutable collection candidates' \
  -e 'constructor\s*\([^)]*(\[\]|Array<|Map<|Set<)' \
  -e 'fn new\([^)]*(Vec<|HashMap<|BTreeMap<|&\[[^]]+\]|&mut)' \
  -e 'def __init__\([^)]*(list|dict|List|Dict)' \
  -e 'func New[A-Za-z]*\([^)]*\[\]' \
  crates web dashboard client-ts client-python client-go
scan_rg 'Protocol count/length candidates' \
  -e 'read_u32_le\(\)\? as usize' \
  -e 'read_u16_le\(\)\? as usize' \
  -e 'Vec::with_capacity\(' \
  -e 'resize\(' \
  -e 'read_chunk\(' \
  -e 'read_raw_frame\(' \
  -e 'read_bytes\(length\)' \
  -e 'for _ in 0\.\.(count|attribute_count|counts_len)' \
  crates/slskr-protocol crates/slskr-client crates/slskr/src
scan_rg 'Protocol scalar emission candidates' \
  -e 'as u(8|16|32|64)' \
  -e '\.len\(\) as u(8|16|32|64)' \
  -e 'write_len_prefixed_bytes' \
  -e 'u32::try_from\([^)]*\.len\(\)' \
  crates/slskr-protocol/src crates/slskr-client/src crates/slskr/src/events_ws.rs crates/slskr/src/storage.rs crates/slskr/src/main.rs
scan_rg 'Resolver/raw stream candidates' \
  -e 'TcpStream|TcpListener|read_exact|read_to_end|read_buf|to_socket_addrs|resolve|connect\(' \
  crates client-go client-python client-ts
scan_rg 'Task/cancellation/lifecycle candidates' \
  -e 'tokio::spawn|spawn\(|abort\(|select!|timeout\(|sleep\(|interval\(|mpsc|broadcast|oneshot' \
  crates web dashboard client-ts client-python client-go
scan_rg 'Example Web API candidates' \
  -e 'localhost:8080|localStorage|window\.open|target="_blank"|Authorization: Bearer|new WebSocket|Access-Control-Allow-Origin' \
  docs README.md web dashboard client-ts client-python client-go
