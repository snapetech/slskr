#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
transport_deriver="scripts/derive-universal-transport-evidence.py"
transport_capability_deriver="scripts/derive-universal-transport-capability-evidence.py"
transport_test="scripts/test-universal-transport-evidence.py"
lifecycle_runner="scripts/run-universal-lifecycle-matrix.py"
lifecycle_test="scripts/test-universal-lifecycle-matrix.py"
status=0

if [[ ! -x "$transport_deriver" || ! -x "$transport_capability_deriver" || ! -x "$transport_test" || ! -x "$lifecycle_runner" || ! -x "$lifecycle_test" ]]; then
  printf 'audit tooling check failed: universal transport/lifecycle evidence tools must be executable\n' >&2
  status=1
fi

scripts/with-process-memory-guard.sh python3 -m py_compile \
  scripts/audit-parity-manifest.py "$transport_deriver" "$transport_capability_deriver" "$transport_test" \
  "$lifecycle_runner" "$lifecycle_test"
scripts/with-process-memory-guard.sh python3 "$transport_test"
scripts/with-process-memory-guard.sh python3 "$lifecycle_test"

if ! rg -n '^\| BUG-022 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'audit tooling check failed: BUG-022 must stay verified in council ledger\n' >&2
  status=1
fi

scripts/with-build-guard.sh cargo metadata --format-version 1 --no-deps >/dev/null
scripts/with-build-guard.sh cargo tree -d >/dev/null

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
