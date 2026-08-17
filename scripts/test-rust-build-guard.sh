#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"
cd "$repo_root"

holder_pid=''
test_lock_path="$(mktemp "${TMPDIR:-/tmp}/slskr-build-guard-lock.XXXXXX")"
holder_log="$(mktemp "${TMPDIR:-/tmp}/slskr-build-guard-test.XXXXXX")"
cleanup() {
  if [[ -n "$holder_pid" ]] && kill -0 "$holder_pid" 2>/dev/null; then
    kill "$holder_pid" 2>/dev/null || true
    wait "$holder_pid" 2>/dev/null || true
  fi
  rm -f "$holder_log" "$test_lock_path"
}
trap cleanup EXIT

SLSKR_BUILD_GUARD_HELD=0 \
SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
SLSKR_BUILD_LOCK_WAIT_SECONDS=0 \
  scripts/with-build-guard.sh bash -c 'sleep 2' >"$holder_log" 2>&1 &
holder_pid=$!
sleep 0.2

if SLSKR_BUILD_GUARD_HELD=0 \
  SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  SLSKR_BUILD_LOCK_WAIT_SECONDS=0 \
  scripts/with-build-guard.sh bash -c 'exit 0'; then
  printf 'Rust build guard test failed: overlapping command was allowed\n' >&2
  exit 1
fi

wait "$holder_pid"
holder_pid=''

guard_limits="$({
  SLSKR_BUILD_GUARD_HELD=0 \
  SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  SLSKR_RUST_BUILD_JOBS=1 \
  RUST_TEST_THREADS=16 scripts/with-build-guard.sh bash -c 'printf "%s %s %s %s %s %s %s %s %s" "$(ulimit -v)" "$CARGO_BUILD_JOBS" "$RUST_TEST_THREADS" "$RUST_MIN_STACK" "$CARGO_PROFILE_DEV_DEBUG" "$CARGO_PROFILE_TEST_DEBUG" "$CARGO_PROFILE_TEST_CODEGEN_UNITS" "$CARGO_PROFILE_TEST_LTO" "$CARGO_INCREMENTAL"'
} 2>/dev/null)"
if [[ "$guard_limits" != "12582912 1 1 16777216 0 0 256 false 0" ]]; then
  printf 'Rust build guard test failed: unexpected inherited limits: %s\n' "$guard_limits" >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  SLSKR_RUST_VIRTUAL_MEMORY_KIB=12582913 \
  scripts/with-build-guard.sh bash -c 'exit 0' >/dev/null 2>&1; then
  printf 'Rust build guard test failed: over-limit virtual memory was accepted\n' >&2
  exit 1
fi

printf 'Rust build guard lock test passed\n'
