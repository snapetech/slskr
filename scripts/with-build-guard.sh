#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"
cd "$repo_root"

if [[ "$#" -eq 0 ]]; then
  printf 'usage: %s <command> [args...]\n' "$BASH_SOURCE" >&2
  exit 2
fi

# A guarded command may invoke another repository script which reaches this
# wrapper again. The outer process owns the lock and the limits are inherited.
if [[ "${SLSKR_BUILD_GUARD_HELD:-0}" == "1" ]]; then
  exec "$@"
fi

lock_wait_seconds="${SLSKR_BUILD_LOCK_WAIT_SECONDS:-0}"
virtual_memory_kib="${SLSKR_RUST_VIRTUAL_MEMORY_KIB:-12582912}"
build_jobs="${SLSKR_RUST_BUILD_JOBS:-1}"
max_virtual_memory_kib=12582912

if [[ ! "$lock_wait_seconds" =~ ^[0-9]{1,5}$ ]]; then
  printf 'SLSKR_BUILD_LOCK_WAIT_SECONDS must be a non-negative integer (seconds)\n' >&2
  exit 2
fi
if [[ ! "$virtual_memory_kib" =~ ^[1-9][0-9]{0,7}$ || "$virtual_memory_kib" -gt "$max_virtual_memory_kib" ]]; then
  printf 'SLSKR_RUST_VIRTUAL_MEMORY_KIB must be between 1 and %s\n' "$max_virtual_memory_kib" >&2
  exit 2
fi
if [[ "$build_jobs" != "1" ]]; then
  printf 'SLSKR_RUST_BUILD_JOBS must be exactly 1; parallel Rust builds are disabled\n' >&2
  exit 2
fi

# The historical monolithic controller test target is known to exceed the
# safe LLVM profile even with the virtual-memory ceiling applied. Refuse the
# two Cargo spellings that enable it before Cargo or rustc starts; the default
# focused test target remains the supported memory-safe path.
if [[ "$1" == "cargo" ]]; then
  cargo_test_requested=0
  full_controller_tests_requested=0
  argument_index=2
  while ((argument_index <= $#)); do
    argument="${!argument_index}"
    if [[ "$argument" == "test" ]]; then
      cargo_test_requested=1
    elif [[ "$argument" == "--all-features" ]]; then
      full_controller_tests_requested=1
    elif [[ "$argument" == "--features" ]]; then
      argument_index=$((argument_index + 1))
      feature_list="${!argument_index:-}"
      feature_list="${feature_list//,/ }"
      read -r -a requested_features <<<"$feature_list"
      for feature in "${requested_features[@]}"; do
        [[ "$feature" == "full-controller-tests" ]] && full_controller_tests_requested=1
      done
    elif [[ "$argument" == --features=* ]]; then
      feature_list="${argument#--features=}"
      feature_list="${feature_list//,/ }"
      read -r -a requested_features <<<"$feature_list"
      for feature in "${requested_features[@]}"; do
        [[ "$feature" == "full-controller-tests" ]] && full_controller_tests_requested=1
      done
    fi
    argument_index=$((argument_index + 1))
  done
  if [[ "$cargo_test_requested" -eq 1 && "$full_controller_tests_requested" -eq 1 ]]; then
    printf 'Rust build guard: full-controller-tests is rejected under the 12 GiB memory-safe profile\n' >&2
    printf 'Rust build guard: use the default focused controller test target\n' >&2
    exit 2
  fi
else
  # Cargo also invokes this file as rustc-wrapper. Reject the feature at that
  # layer too, so a direct Cargo test invocation cannot bypass the outer command-line
  # check through .cargo/config.toml.
  expect_cfg_value=0
  for argument in "$@"; do
    if [[ "$expect_cfg_value" -eq 1 ]]; then
      if [[ "$argument" == 'feature="full-controller-tests"' || "$argument" == 'feature=full-controller-tests' ]]; then
        printf 'Rust build guard: full-controller-tests is rejected under the 12 GiB memory-safe profile\n' >&2
        printf 'Rust build guard: use the default focused controller test target\n' >&2
        exit 2
      fi
      expect_cfg_value=0
    fi
    if [[ "$argument" == "--cfg" ]]; then
      expect_cfg_value=1
    elif [[ "$argument" == '--cfg=feature="full-controller-tests"' || "$argument" == "--cfg=feature=full-controller-tests" ]]; then
      printf 'Rust build guard: full-controller-tests is rejected under the 12 GiB memory-safe profile\n' >&2
      printf 'Rust build guard: use the default focused controller test target\n' >&2
      exit 2
    fi
  done
fi

mkdir -p "$repo_root/target"
lock_path="${SLSKR_BUILD_LOCK_PATH:-$repo_root/target/.slskr-rust-build.lock}"
if command -v flock >/dev/null 2>&1; then
  exec 9>"$lock_path"
  if [[ "$lock_wait_seconds" == "0" ]]; then
    if ! flock -n 9; then
      printf 'Rust build guard: another guarded Rust command is active; refusing to overlap it\n' >&2
      printf 'Rust build guard: retry after it exits or set SLSKR_BUILD_LOCK_WAIT_SECONDS to a bounded value\n' >&2
      exit 75
    fi
  elif ! flock -w "$lock_wait_seconds" 9; then
    printf 'Rust build guard: timed out waiting for the repository Rust lock\n' >&2
    exit 75
  fi
else
  # Git Bash on Windows may not provide flock. mkdir is atomic, so use it as
  # the portable fallback and remove only a proven-dead owner lock.
  lock_dir="$lock_path.d"
  release_mkdir_lock() {
    rm -f "$lock_dir/pid"
    rmdir "$lock_dir" 2>/dev/null || true
  }
  acquired=0
  deadline=$((SECONDS + lock_wait_seconds))
  while [[ "$acquired" -eq 0 ]]; do
    if mkdir "$lock_dir" 2>/dev/null; then
      printf '%s\n' "$$" >"$lock_dir/pid"
      trap release_mkdir_lock EXIT
      trap 'release_mkdir_lock; exit 130' INT TERM
      acquired=1
    else
      owner_pid=''
      if [[ -f "$lock_dir/pid" ]]; then
        owner_pid="$(<"$lock_dir/pid")"
        if [[ "$owner_pid" =~ ^[0-9]+$ ]] && ! kill -0 "$owner_pid" 2>/dev/null; then
          rm -f "$lock_dir/pid"
          rmdir "$lock_dir" 2>/dev/null || true
          continue
        fi
      fi
      if [[ "$lock_wait_seconds" == "0" || "$SECONDS" -ge "$deadline" ]]; then
        printf 'Rust build guard: another guarded Rust command is active; refusing to overlap it\n' >&2
        printf 'Rust build guard: retry after it exits or set SLSKR_BUILD_LOCK_WAIT_SECONDS to a bounded value\n' >&2
        exit 75
      fi
      sleep 1
    fi
  done
fi

printf '[rust-build-guard] lock=exclusive jobs=1 virtual-memory=%s KiB command=' "$virtual_memory_kib" >&2
printf ' %q' "$@" >&2
printf '\n' >&2

(
  ulimit -v "$virtual_memory_kib"
  export CARGO_BUILD_JOBS=1
  # The large slskr binary can make LLVM retain several gigabytes of debug
  # metadata and incremental state even with one rustc process. Keep those
  # defaults disabled inside the guard so a build stays below the repository
  # ceiling instead of exhausting the host while LLVM is linking.
  export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}"
  export CARGO_PROFILE_TEST_DEBUG="${CARGO_PROFILE_TEST_DEBUG:-0}"
  export CARGO_PROFILE_TEST_CODEGEN_UNITS="${CARGO_PROFILE_TEST_CODEGEN_UNITS:-256}"
  export CARGO_PROFILE_TEST_LTO="${CARGO_PROFILE_TEST_LTO:-false}"
  export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
  export RUST_TEST_THREADS=1
  export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
  export SLSKR_BUILD_GUARD_HELD=1
  exec "$@"
)
