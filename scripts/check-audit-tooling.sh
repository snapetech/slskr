#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-022 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'audit tooling check failed: BUG-022 must stay verified in council ledger\n' >&2
  status=1
fi

cargo metadata --format-version 1 --no-deps >/dev/null
cargo tree -d >/dev/null

for expected in 'cargo metadata --format-version 1 --no-deps' 'cargo tree -d' 'cargo audit'; do
  if ! rg -n -F "$expected" scripts docs .github >/dev/null; then
    printf 'audit tooling check failed: expected audit token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n -F 'scripts/check-audit-tooling.sh' scripts/check-remediation-baseline.sh docs/dev/bug-burndown-ledger.md >/dev/null; then
  printf 'audit tooling check failed: audit tooling gate is not registered\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'audit tooling check passed\n'
