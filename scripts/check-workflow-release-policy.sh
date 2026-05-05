#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-002 BUG-012 BUG-014 BUG-016 BUG-023; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'workflow release policy check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

for expected in 'ACTIONLINT_VERSION: v' 'concurrency:' 'attest-build-provenance@' 'attestations: write' 'id-token: write'; do
  if ! rg -n -F "$expected" .github/workflows >/dev/null; then
    printf 'workflow release policy check failed: expected workflow hardening token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if rg -n 'go install "github.com/rhysd/actionlint/cmd/actionlint@latest"|go install github.com/rhysd/actionlint/cmd/actionlint@latest' .github/workflows; then
  printf 'workflow release policy check failed: actionlint install must stay pinned\n' >&2
  status=1
fi

if ! rg -n -F "release-*" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release tag trigger was not found\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'workflow release policy check passed\n'
