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

if ! rg -n 'trusted_proxy_rate_limit_addr_rejects_spoofed_leftmost_hop' crates/slskr/src/main.rs >/dev/null ||
   ! rg -n 'trusted_proxy_rate_limit_addr_fails_closed_on_malformed_chain' crates/slskr/src/main.rs >/dev/null ||
   ! rg -n 'trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header' crates/slskr/src/main.rs >/dev/null; then
  printf 'rate-limit proxy policy check failed: spoofing rejection coverage is missing\n' >&2
  status=1
fi

if ! rg -n 'BUG-008 .*Verified' docs/dev/bug-burndown-ledger.md >/dev/null; then
  printf 'rate-limit proxy policy check failed: BUG-008 must be marked verified in the council ledger\n' >&2
  status=1
fi

if ! rg -n --fixed-strings -- 'authenticated_rate_limit_user_key' crates/slskr/src/main.rs >/dev/null ||
   ! rg -n --fixed-strings -- 'if !is_authorized(config, authorization, cookie)' crates/slskr/src/main.rs >/dev/null ||
   ! rg -n --fixed-strings -- '.map(|token| rate_limit_user_key(&token))' crates/slskr/src/main.rs >/dev/null ||
   ! rg -n --fixed-strings -- 'authenticated_rate_limit_key_uses_verified_credential_identity' crates/slskr/src/main.rs >/dev/null; then
  printf 'rate-limit proxy policy check failed: verified authenticated rate-limit identity is missing\n' >&2
  status=1
fi

if rg -n 'rate_limit_user_key\((authorization|cookie)' crates/slskr/src/main.rs >/dev/null; then
  printf 'rate-limit proxy policy check failed: rate-limit identity must hash only a verified credential\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rate-limit proxy policy check passed\n'
