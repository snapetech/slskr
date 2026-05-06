#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

inventory="docs/dev/council-scan-inventory.md"
ledger="docs/dev/bug-burndown-ledger.md"
status=0

if [[ ! -f "$inventory" ]]; then
  printf 'council loop check failed: missing %s\n' "$inventory" >&2
  exit 1
fi

for expected in \
  'Remaining Candidate Classes' \
  'Constructor/mutable collection candidates' \
  'Protocol count/length candidates' \
  'Protocol scalar emission candidates' \
  'Resolver/raw stream candidates' \
  'Task/cancellation/lifecycle candidates' \
  'Example Web API candidates' \
  'Fixed' \
  'Existing Guard' \
  'False Positive' \
  'Accepted' \
  'Unclassified'
do
  if ! rg -n -F "$expected" "$inventory" >/dev/null; then
    printf 'council loop check failed: inventory missing required token: %s\n' "$expected" >&2
    status=1
  fi
done

for expected in \
  'scripts/run-council-scan.sh' \
  'scripts/run-council-bughunt.sh' \
  'scripts/check-council-loop.sh' \
  'scripts/check-rust-protocol-taint-lens.sh' \
  'scripts/check-rust-protocol-adversarial-corpus.sh' \
  'docs/dev/council-bughunt-playbook.md' \
  'New | Candidate is plausible but not yet accepted as a real bug.'
do
  if ! rg -n -F "$expected" "$inventory" "$ledger" docs/dev/council-bughunt-playbook.md scripts/check-council-loop.sh scripts/check-remediation-baseline.sh >/dev/null; then
    printf 'council loop check failed: required process token missing: %s\n' "$expected" >&2
    status=1
  fi
done

for id in BUG-031 BUG-040; do
  if ! rg -n "^\\| $id .* \\| Verified \\|$" "$ledger" >/dev/null; then
    printf 'council loop check failed: %s must stay verified in council ledger\n' "$id" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'council loop check passed\n'
