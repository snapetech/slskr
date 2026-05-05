#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-030 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'docs freshness check failed: BUG-030 must stay verified in council ledger\n' >&2
  status=1
fi

if rg -n 'http_api_|slskr:latest|WebSocket connections are not currently supported' \
  docs/http-api.md docs/INTEGRATION_GUIDE.md docs/http-api-features.md docs/CLIENT_LIBRARIES.md >/dev/null; then
  printf 'docs freshness check failed: stale API/config/image/WebSocket guidance remains\n' >&2
  rg -n 'http_api_|slskr:latest|WebSocket connections are not currently supported' \
    docs/http-api.md docs/INTEGRATION_GUIDE.md docs/http-api-features.md docs/CLIENT_LIBRARIES.md >&2
  status=1
fi

if rg -n 'Access-Control-Allow-Origin:\s*\*' docs --glob '*.md' \
  --glob '!docs/http-api-deployment.md' --glob '!docs/security-bug-burndown.md' >/dev/null; then
  printf 'docs freshness check failed: wildcard CORS examples must stay out of general docs\n' >&2
  rg -n 'Access-Control-Allow-Origin:\s*\*' docs --glob '*.md' \
    --glob '!docs/http-api-deployment.md' --glob '!docs/security-bug-burndown.md' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'docs freshness check passed\n'
