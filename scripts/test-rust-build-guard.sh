#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

expected_virtual_memory="12582912"
if [[ "$(uname -s)" == "Darwin" ]]; then
  expected_virtual_memory="unlimited"
fi

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
  RUST_TEST_THREADS=16 scripts/with-build-guard.sh bash -c 'printf "%s %s %s %s %s %s %s %s %s %s" "$(ulimit -v)" "$CARGO_BUILD_JOBS" "$RUST_TEST_THREADS" "$RUST_MIN_STACK" "$CARGO_PROFILE_DEV_DEBUG" "$CARGO_PROFILE_DEV_CODEGEN_UNITS" "$CARGO_PROFILE_TEST_DEBUG" "$CARGO_PROFILE_TEST_CODEGEN_UNITS" "$CARGO_PROFILE_TEST_LTO" "$CARGO_INCREMENTAL"'
} 2>/dev/null)"
if [[ "$guard_limits" != "$expected_virtual_memory 1 1 16777216 0 1024 0 256 false 0" ]]; then
  printf 'Rust build guard test failed: unexpected inherited limits: %s\n' "$guard_limits" >&2
  exit 1
fi

nested_guard_limit="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
  SLSKR_PROCESS_MEMORY_MAX_KIB=262144 \
  SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
    scripts/with-process-memory-guard.sh scripts/with-build-guard.sh bash -c 'ulimit -v' 2>/dev/null
)"
expected_nested_guard_limit="262144"
if [[ "$(uname -s)" == "Darwin" ]]; then
  expected_nested_guard_limit="unlimited"
fi
if [[ "$nested_guard_limit" != "$expected_nested_guard_limit" ]]; then
  printf 'Rust build guard test failed: nested process limit was raised or lost: %s\n' "$nested_guard_limit" >&2
  exit 1
fi

if command -v systemd-run >/dev/null 2>&1 \
  && command -v systemctl >/dev/null 2>&1 \
  && systemctl --user show-environment >/dev/null 2>&1; then
  nested_cargo_output="$({
    scripts/with-process-memory-guard.sh bash -c \
      'scripts/with-build-guard.sh cargo metadata --format-version 1 --no-deps >/dev/null'
  } 2>&1)"
  if ! rg -q 'slskr-rust-build-guard-' <<<"$nested_cargo_output"; then
    printf 'Rust build guard test failed: nested Cargo did not move to its Rust-specific systemd unit\n' >&2
    exit 1
  fi
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  SLSKR_RUST_VIRTUAL_MEMORY_KIB=12582913 \
  scripts/with-build-guard.sh bash -c 'exit 0' >/dev/null 2>&1; then
  printf 'Rust build guard test failed: over-limit virtual memory was accepted\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  scripts/with-build-guard.sh cargo test -p slskr --features full-controller-tests --no-run >/dev/null 2>&1; then
  printf 'Rust build guard test failed: monolithic controller tests were allowed\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  scripts/with-build-guard.sh cargo test --all-features --no-run >/dev/null 2>&1; then
  printf 'Rust build guard test failed: all-features tests were allowed\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  SLSKR_ALLOW_FULL_CONTROLLER_TESTS=1 \
  scripts/with-build-guard.sh cargo test -p slskr --features full-controller-tests --no-run >/dev/null 2>&1; then
  printf 'Rust build guard test failed: unguarded full-controller-tests opt-in was allowed\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  scripts/with-build-guard.sh /bin/true --cfg 'feature="full-controller-tests"' >/dev/null 2>&1; then
  printf 'Rust build guard test failed: compiler-wrapper feature bypass was allowed\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  scripts/with-build-guard.sh cargo build -p slskr --features legacy-route-dispatch --no-default-features >/dev/null 2>&1; then
  printf 'Rust build guard test failed: legacy route dispatcher was allowed\n' >&2
  exit 1
fi

if SLSKR_BUILD_LOCK_PATH="$test_lock_path" \
  scripts/with-build-guard.sh /bin/true --cfg 'feature="legacy-route-dispatch"' >/dev/null 2>&1; then
  printf 'Rust build guard test failed: legacy route dispatcher wrapper bypass was allowed\n' >&2
  exit 1
fi

printf 'Rust build guard lock test passed\n'
