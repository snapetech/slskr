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
    --glob '!client-ts/node_modules/**' "$@" >"$tmp" || true
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
  -e 'count|length|len\(|with_capacity|resize|reserve|read_u(16|32|64)|read_i(16|32|64)' \
  crates/slskr-protocol crates/slskr-client crates/slskr/src
scan_rg 'Protocol scalar emission candidates' \
  -e 'write_u(8|16|32|64)|write_i(8|16|32|64)|write_string|as u(8|16|32|64)|\.len\(\) as' \
  crates/slskr-protocol crates/slskr-client crates/slskr/src
scan_rg 'Resolver/raw stream candidates' \
  -e 'TcpStream|TcpListener|read_exact|read_to_end|read_buf|to_socket_addrs|resolve|connect\(' \
  crates client-go client-python client-ts
scan_rg 'Task/cancellation/lifecycle candidates' \
  -e 'tokio::spawn|spawn\(|abort\(|select!|timeout\(|sleep\(|interval\(|mpsc|broadcast|oneshot' \
  crates web dashboard client-ts client-python client-go
scan_rg 'Example Web API candidates' \
  -e 'localhost:8080|localStorage|window\.open|target="_blank"|Authorization: Bearer|new WebSocket|Access-Control-Allow-Origin' \
  docs README.md web dashboard client-ts client-python client-go
