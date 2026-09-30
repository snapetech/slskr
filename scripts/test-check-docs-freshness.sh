#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/slskr-docs-freshness.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

maintained_files=(
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
  client-go/examples/advanced_usage.go
  client-go/examples/basic_usage.go
  client-go/examples/integration_example.go
  client-go/examples/websocket_events.go
  client-python/examples/advanced_usage.py
  client-python/examples/basic_usage.py
  client-python/examples/integration_example.py
  client-python/examples/websocket_events.py
  client-ts/examples/basic-usage.ts
)

for path in "${maintained_files[@]}"; do
  mkdir -p "$tmp_root/$(dirname "$path")"
  cp "$repo_root/$path" "$tmp_root/$path"
done
mkdir -p "$tmp_root/docs/dev"
printf '%s\n' '| BUG-030 docs freshness | Docs | Low | High | Evidence | Impact | Verified |' \
  > "$tmp_root/docs/dev/bug-burndown-ledger.md"

SLSKR_DOCS_FRESHNESS_ROOT="$tmp_root" \
  "$repo_root/scripts/check-docs-freshness.sh" >/dev/null

printf '\nslskr:latest\n' >> "$tmp_root/examples/README.md"
if SLSKR_DOCS_FRESHNESS_ROOT="$tmp_root" \
  "$repo_root/scripts/check-docs-freshness.sh" >/dev/null 2>&1; then
  printf 'docs freshness test failed: stale guidance was not rejected\n' >&2
  exit 1
fi

printf 'docs freshness tests passed\n'
