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

for expected in 'scripts/run-council-scan.sh' 'scripts/check-council-loop.sh' 'New | Candidate is plausible but not yet accepted as a real bug.'; do
  if ! rg -n -F "$expected" "$inventory" "$ledger" scripts/check-remediation-baseline.sh >/dev/null; then
    printf 'council loop check failed: required process token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n '^\| BUG-031 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'council loop check failed: BUG-031 must stay verified in council ledger\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'council loop check passed\n'
