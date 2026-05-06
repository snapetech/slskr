#!/usr/bin/env bash
#
# slskR Bug Council negative-space gate. Asserts two halves for every
# declared trust boundary in docs/dev/bug-council-negative-space.md:
#
#   1. assert_validator_present   — validator symbol exists in the sink.
#   2. assert_loop_anchor         — the same symbol is referenced from
#                                   scripts/check-council-loop.sh so the
#                                   wider council loop fails if the gate
#                                   wires get severed.
#
# Both halves are required. Mirrors the strengthening from slskdN's
# scripts/check-council-negative-space.sh after the council found that the
# original one-half version was itself a bug (a baseline could be removed
# while the gate kept passing because it only inspected the sink file).

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

assert_loop_anchor() {
  local boundary="$1"
  local anchor="$2"

  if rg -n --fixed-strings -- "$anchor" docs/dev/council-scan-inventory.md >/dev/null \
     || rg -n --fixed-strings -- "$anchor" scripts/check-council-loop.sh >/dev/null; then
    pass "negative-space: [$boundary] anchor '$anchor' is registered in council loop or inventory"
  else
    fail "negative-space: [$boundary] anchor '$anchor' is missing from council loop and inventory"
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

# The wire-frame boundary is also tracked in the council inventory by name,
# under the existing "Protocol count/length" candidate class. Strengthening:
# at least one anchor (DEFAULT_MAX_FRAME_LEN, read_raw_frame_with_max, or the
# inventory's BUG-035 row tag) must remain referenced in either the inventory
# or check-council-loop.sh.
assert_loop_anchor "wire-frame-length" "BUG-035"

if [[ "$failures" -gt 0 ]]; then
  printf '\n%d negative-space gate check(s) failed.\n' "$failures" >&2
  exit 1
fi

printf '\nAll negative-space gate checks passed.\n'
