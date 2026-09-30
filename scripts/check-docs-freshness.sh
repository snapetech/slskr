#!/usr/bin/env bash
set -euo pipefail

repo_root="${SLSKR_DOCS_FRESHNESS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

maintained_docs=(
  docs/http-api.md
  docs/INTEGRATION_GUIDE.md
  docs/http-api-features.md
  docs/CLIENT_LIBRARIES.md
  docs/live-interop-test-matrix.md
  examples/README.md
  client-go/README.md
  client-python/README.md
  client-ts/README.md
  web/e2e/README.md
  test-data/slskr-test-fixtures/README.md
)

while IFS= read -r path; do
  maintained_docs+=("$path")
done < <(printf '%s\n' \
  client-go/examples/*.go \
  client-python/examples/*.py \
  client-ts/examples/*.ts)

for path in "${maintained_docs[@]}"; do
  if [[ ! -f "$path" ]]; then
    printf 'docs freshness check failed: maintained file is missing: %s\n' "$path" >&2
    status=1
  fi
done

if [[ ! -f "$ledger" ]]; then
  printf 'docs freshness check failed: council ledger is missing: %s\n' "$ledger" >&2
  status=1
fi

if [[ -f "$ledger" ]] && ! rg -n '^\| BUG-030 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'docs freshness check failed: BUG-030 must stay verified in council ledger\n' >&2
  status=1
fi

stale_patterns=(
  'http_api_'
  'slskr:latest'
  'WebSocket connections are not currently supported'
  'subprotocols=\["chat"\]'
  'search_updates'
  'transfer_updates'
  'HTTP_API_BEARER_TOKEN'
  'cd src/web'
  'examples/basic-usage\.js'
  'examples/(search_monitor|transfer_manager|message_broadcaster)\.'
  'client\.search\.create'
  'client\.messages\.send'
)
stale_pattern=''
for pattern in "${stale_patterns[@]}"; do
  if [[ -n "$stale_pattern" ]]; then
    stale_pattern+="|"
  fi
  stale_pattern+="($pattern)"
done

if rg -n "$stale_pattern" "${maintained_docs[@]}" >/dev/null; then
  printf 'docs freshness check failed: stale SDK/example/API/WebSocket guidance remains\n' >&2
  rg -n "$stale_pattern" "${maintained_docs[@]}" >&2
  status=1
fi

if rg -n 'Access-Control-Allow-Origin:\s*\*' "$repo_root/docs" --glob '*.md' \
  --glob '!docs/http-api-deployment.md' --glob '!docs/security-bug-burndown.md' >/dev/null; then
  printf 'docs freshness check failed: wildcard CORS examples must stay out of general docs\n' >&2
  rg -n 'Access-Control-Allow-Origin:\s*\*' "$repo_root/docs" --glob '*.md' \
    --glob '!docs/http-api-deployment.md' --glob '!docs/security-bug-burndown.md' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'docs freshness check passed\n'
