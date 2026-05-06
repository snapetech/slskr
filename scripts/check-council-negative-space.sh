#!/usr/bin/env bash
#
# slskR Bug Council negative-space gate. Asserts that every declared trust
# boundary in docs/dev/bug-council-negative-space.md still has its required
# validator symbol present in the expected sink file.
#
# Complementary to scripts/run-council-scan.sh (which inventories existing
# suspicious call sites) and scripts/check-council-loop.sh (which gates sweep
# closure). This gate inverts the search: declarative list of boundaries.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0

pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; failures=$((failures + 1)); }

assert_validator_present() {
  local boundary="$1"
  local sink="$2"
  local symbol="$3"

  if [[ ! -e "$sink" ]]; then
    fail "negative-space: sink missing for boundary [$boundary]: $sink"
    return
  fi

  if rg -n --fixed-strings -- "$symbol" "$sink" >/dev/null; then
    pass "negative-space: [$boundary] $symbol present in $sink"
  else
    fail "negative-space: [$boundary] $symbol missing from $sink"
  fi
}

# Wire-frame length bounds.
assert_validator_present \
  "wire-frame-length" \
  "crates/slskr-client/src/io.rs" \
  "DEFAULT_MAX_FRAME_LEN"

assert_validator_present \
  "wire-frame-length" \
  "crates/slskr-client/src/io.rs" \
  "read_raw_frame_with_max"

if [[ "$failures" -gt 0 ]]; then
  printf '\n%d negative-space gate check(s) failed.\n' "$failures" >&2
  exit 1
fi

printf '\nAll negative-space gate checks passed.\n'
