#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

if ! rg -n 'SLSKR_TRUSTED_PROXY_CIDRS|trusted_proxy_cidrs' crates/slskr/src docs >/dev/null; then
  printf 'rate-limit proxy policy check failed: trusted proxy CIDR configuration is missing\n' >&2
  status=1
fi

if ! rg -n 'x_forwarded_for|forwarded_header_client_ip|x_forwarded_for_client_ip' crates/slskr/src/main.rs crates/slskr/src/http_server.rs >/dev/null; then
  printf 'rate-limit proxy policy check failed: Forwarded/X-Forwarded-For parsing is missing\n' >&2
  status=1
fi

if ! rg -n 'trusted_proxy_cidrs.*any|rate_limit_remote_addr' crates/slskr/src/main.rs >/dev/null; then
  printf 'rate-limit proxy policy check failed: rate limiter must only trust forwarded headers from allowlisted proxies\n' >&2
  status=1
fi

if ! rg -n 'spoof|raw peer|untrusted' crates/slskr/src/main.rs docs/dev/bug-burndown-ledger.md >/dev/null; then
  printf 'rate-limit proxy policy check failed: spoofing rejection coverage is missing\n' >&2
  status=1
fi

if ! rg -n 'BUG-008 .*Verified' docs/dev/bug-burndown-ledger.md >/dev/null; then
  printf 'rate-limit proxy policy check failed: BUG-008 must be marked verified in the council ledger\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rate-limit proxy policy check passed\n'
