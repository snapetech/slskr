#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$#" -eq 0 ]]; then
  printf 'usage: %s <command> [args...]\n' "${BASH_SOURCE[0]}" >&2
  exit 2
fi

lock_wait_seconds="${SLSKR_BUILD_LOCK_WAIT_SECONDS:-0}"
virtual_memory_kib="${SLSKR_RUST_VIRTUAL_MEMORY_KIB:-12582912}"
build_jobs="${SLSKR_RUST_BUILD_JOBS:-1}"
max_virtual_memory_kib=12582912
host_platform="$(uname -s 2>/dev/null || printf 'unknown')"
virtual_memory_limit_supported=1
if [[ "$host_platform" == "Darwin" ]]; then
  # Darwin's shell does not expose a settable RLIMIT_AS through `ulimit -v`.
  # Keep the exclusive Rust lock and reduced compiler profile, but do not make
  # every macOS Cargo command fail before Cargo starts.
  virtual_memory_limit_supported=0
fi

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

# Cargo's `fmt --check` path invokes rustfmt's diff emitter over the entire
# workspace. This repository contains a multi-megabyte monolithic controller
# source whose diff path has previously attempted a host-sized allocation.
# Refuse the subcommand before Cargo or rustfmt starts; the incremental checker
# uses `--emit stdout` behind its dedicated formatter guard instead.
cargo_fmt_requested=0
if [[ "$1" == "cargo" ]]; then
  cargo_argument_index=2
  while ((cargo_argument_index <= $#)); do
    cargo_argument="${!cargo_argument_index}"
    case "$cargo_argument" in
      +*)
        ;;
      --color|--config|--manifest-path|--jobs)
        cargo_argument_index=$((cargo_argument_index + 1))
        ;;
      --color=*|--config=*|--manifest-path=*|--jobs=*|--locked|--offline|--frozen|--verbose|--quiet)
        ;;
      -*)
        ;;
      fmt)
        cargo_fmt_requested=1
        break
        ;;
      *)
        break
        ;;
    esac
    cargo_argument_index=$((cargo_argument_index + 1))
  done
fi
if [[ "$cargo_fmt_requested" -eq 1 ]]; then
  printf 'Rust build guard: cargo fmt is disabled for this repository\n' >&2
  printf 'Rust build guard: use scripts/check-rust-format.sh; it formats one changed file at a time under a 1 GiB cap\n' >&2
  exit 2
fi

# This marker is only a nesting hint. It must never let a caller skip the
# limit, lock, Cargo-profile, or feature checks. Validate that the current
# process is actually inside the bounded context before using it to suppress
# setup. An environment variable by itself is not proof of an active guard.
build_guard_context_active=0
if [[ "${SLSKR_BUILD_GUARD_HELD:-0}" == "1" ]]; then
  inherited_build_virtual_memory_kib="unlimited"
  if [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
    inherited_build_virtual_memory_kib="$(ulimit -v)"
  fi
  if [[ "$inherited_build_virtual_memory_kib" =~ ^[0-9]+$ ]] \
    && ((inherited_build_virtual_memory_kib <= virtual_memory_kib)); then
    build_guard_context_active=1
  elif grep -Eq '/slskr-rust-build-guard-[^/]+\.service' /proc/self/cgroup 2>/dev/null; then
    build_guard_context_active=1
  fi
fi
if [[ "$build_guard_context_active" -eq 1 ]]; then
  nested_build_guard=1
else
  nested_build_guard=0
fi

process_memory_guard_context_active=0
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_HELD:-0}" == "1" ]]; then
  inherited_process_virtual_memory_kib="unlimited"
  if [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
    inherited_process_virtual_memory_kib="$(ulimit -v)"
  fi
  if [[ "$inherited_process_virtual_memory_kib" =~ ^[0-9]+$ ]] \
    && ((inherited_process_virtual_memory_kib <= 4194304)); then
    process_memory_guard_context_active=1
  elif grep -Eq '/slskr-process-memory-guard-[^/]+\.service' /proc/self/cgroup 2>/dev/null; then
    process_memory_guard_context_active=1
  fi
fi

# The historical monolithic controller test target is known to exceed the
# safe LLVM profile even with the virtual-memory ceiling applied. Refuse the
# two Cargo spellings that enable it before Cargo or rustc starts; the default
# focused test target remains the supported memory-safe path. An explicit
# opt-in is accepted only when the outer process-memory guard owns the command,
# so this exception can never create an unbounded full-suite build.
if [[ "$1" == "cargo" ]]; then
  cargo_test_requested=0
  full_controller_tests_requested=0
  legacy_route_dispatch_requested=0
  full_controller_tests_override=0
  if [[ "${SLSKR_ALLOW_FULL_CONTROLLER_TESTS:-0}" == "1" \
    && "$process_memory_guard_context_active" -eq 1 \
    && "${SLSKR_BUILD_GUARD_AUTO_PROCESS_GUARD:-0}" != "1" ]]; then
    full_controller_tests_override=1
  fi
  argument_index=2
  while ((argument_index <= $#)); do
    argument="${!argument_index}"
    if [[ "$argument" == "test" ]]; then
      cargo_test_requested=1
    elif [[ "$argument" == "--all-features" ]]; then
      full_controller_tests_requested=1
      legacy_route_dispatch_requested=1
    elif [[ "$argument" == "--features" ]]; then
      argument_index=$((argument_index + 1))
      feature_list="${!argument_index:-}"
      feature_list="${feature_list//,/ }"
      read -r -a requested_features <<<"$feature_list"
      for feature in "${requested_features[@]}"; do
        [[ "$feature" == "full-controller-tests" ]] && full_controller_tests_requested=1
        [[ "$feature" == "legacy-route-dispatch" ]] && legacy_route_dispatch_requested=1
      done
    elif [[ "$argument" == --features=* ]]; then
      feature_list="${argument#--features=}"
      feature_list="${feature_list//,/ }"
      read -r -a requested_features <<<"$feature_list"
      for feature in "${requested_features[@]}"; do
        [[ "$feature" == "full-controller-tests" ]] && full_controller_tests_requested=1
        [[ "$feature" == "legacy-route-dispatch" ]] && legacy_route_dispatch_requested=1
      done
    fi
    argument_index=$((argument_index + 1))
  done
  if [[ "$legacy_route_dispatch_requested" -eq 1 ]]; then
    printf 'Rust build guard: legacy-route-dispatch is rejected; use the bounded default dispatcher\n' >&2
    exit 2
  fi
  if [[ "$cargo_test_requested" -eq 1 \
    && "$full_controller_tests_requested" -eq 1 \
    && "$full_controller_tests_override" -ne 1 ]]; then
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
      if [[ "$argument" == 'feature="legacy-route-dispatch"' || "$argument" == 'feature=legacy-route-dispatch' ]]; then
        printf 'Rust build guard: legacy-route-dispatch is rejected; use the bounded default dispatcher\n' >&2
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
    elif [[ "$argument" == '--cfg=feature="legacy-route-dispatch"' || "$argument" == "--cfg=feature=legacy-route-dispatch" ]]; then
      printf 'Rust build guard: legacy-route-dispatch is rejected; use the bounded default dispatcher\n' >&2
      exit 2
    fi
  done
fi

# Resolve Cargo before entering the guarded command. A workstation-level shim
# routes bare `cargo` here, so invoking Cargo by name again would recurse into
# the shim. rustup provides the canonical toolchain binary; a normal PATH
# lookup remains available for minimal CI/container environments.
guarded_command=("$@")
if [[ "$1" == "cargo" ]]; then
  cargo_binary=""
  if command -v rustup >/dev/null 2>&1; then
    cargo_binary="$(rustup which cargo 2>/dev/null || true)"
  fi
  if [[ -z "$cargo_binary" ]]; then
    cargo_binary="$(command -v cargo || true)"
  fi
  if [[ -z "$cargo_binary" || ! -x "$cargo_binary" ]]; then
    printf 'Rust build guard: unable to resolve the real Cargo binary\n' >&2
    exit 127
  fi
  resolved_cargo_binary="$(readlink -f "$cargo_binary" 2>/dev/null || printf '%s' "$cargo_binary")"
  if [[ "$resolved_cargo_binary" == "$repo_root/scripts/rust-tool-shim.sh" ]]; then
    printf 'Rust build guard: Cargo resolved to the repository shim instead of the toolchain binary\n' >&2
    exit 127
  fi
  guarded_command=("$cargo_binary" "${@:2}")
fi

lock_path="${SLSKR_BUILD_LOCK_PATH:-$repo_root/target/.slskr-rust-build.lock}"
if [[ "$nested_build_guard" -eq 0 ]]; then
  mkdir -p "$repo_root/target"
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
fi

printf '[rust-build-guard] lock=exclusive jobs=1 virtual-memory=%s KiB command=' "$virtual_memory_kib" >&2
printf ' %q' "${guarded_command[@]}" >&2
printf '\n' >&2

# Export the bounded build profile before taking an environment snapshot for a
# sibling systemd unit. Direct Rust invocations and re-homed Cargo commands
# must receive the same serialization and compiler-working-set settings.
export CARGO_BUILD_JOBS=1
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="${CARGO_PROFILE_DEV_CODEGEN_UNITS:-1024}"
export CARGO_PROFILE_TEST_DEBUG="${CARGO_PROFILE_TEST_DEBUG:-0}"
export CARGO_PROFILE_TEST_CODEGEN_UNITS="${CARGO_PROFILE_TEST_CODEGEN_UNITS:-256}"
export CARGO_PROFILE_TEST_LTO="${CARGO_PROFILE_TEST_LTO:-false}"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export RUST_TEST_THREADS=1
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"

# On a systemd user manager, every top-level Cargo command moves into a
# sibling Rust-specific unit with a hard resident-memory and no-swap ceiling.
# This applies to direct builds as well as Cargo discovered inside the
# application guard. The child inherits the build settings while marking the
# guard held so compiler-wrapper calls do not recursively create units.
if [[ "$1" == "cargo" \
  && "$nested_build_guard" -eq 0 \
  && "${SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD:-0}" != "1" ]] \
  && command -v systemd-run >/dev/null 2>&1 \
  && command -v systemctl >/dev/null 2>&1 \
  && systemctl --user show-environment >/dev/null 2>&1; then
  build_unit_name="slskr-rust-build-guard-${BASHPID}-${RANDOM}.service"
  build_environment_file=""
  cleanup_build_unit() {
    local status="$?"
    trap - EXIT INT TERM
    systemctl --user stop "$build_unit_name" >/dev/null 2>&1 || true
    if [[ -n "$build_environment_file" ]]; then
      rm -f -- "$build_environment_file"
    fi
    exit "$status"
  }
  trap cleanup_build_unit EXIT INT TERM
  build_environment_file="$(mktemp "${TMPDIR:-/tmp}/slskr-rust-build-guard-environment.XXXXXX")"
  chmod 600 "$build_environment_file"
  while IFS= read -r -d '' environment_entry; do
    case "$environment_entry" in
      SLSKR_BUILD_GUARD_HELD=*)
        continue
        ;;
    esac
    environment_name="${environment_entry%%=*}"
    environment_value="${environment_entry#*=}"
    printf -v escaped_environment_value '%q' "$environment_value"
    printf '%s=%s\n' "$environment_name" "$escaped_environment_value" >> "$build_environment_file"
  done < <(env -0)
  systemd-run --user --wait --pipe --collect \
    --unit="$build_unit_name" \
    --working-directory="$repo_root" \
    --property="MemoryMax=${virtual_memory_kib}K" \
    --property="MemorySwapMax=0" \
    --property="TasksMax=512" \
    --property="EnvironmentFile=$build_environment_file" \
    --setenv=SLSKR_BUILD_GUARD_HELD=1 \
    "${guarded_command[@]}"
  exit "$?"
fi

# The fallback shell guard cannot enlarge an inherited virtual-memory hard
# limit. Refuse a Cargo build in that situation instead of waiting for the
# kernel to kill an arbitrary compiler process at the smaller application
# limit. The direct nested-guard test uses a non-Cargo command and remains
# covered by the inherited-limit behavior.
inherited_virtual_memory_kib="unlimited"
if [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
  inherited_virtual_memory_kib="$(ulimit -v)"
fi
if [[ "$1" == "cargo" \
  && "$process_memory_guard_context_active" -eq 1 \
  && "$inherited_virtual_memory_kib" =~ ^[0-9]+$ \
  && "$inherited_virtual_memory_kib" -lt "$virtual_memory_kib" ]]; then
  printf 'Rust build guard: Cargo is inside a smaller process-memory limit (%s KiB); build before entering the application guard or use a systemd user manager\n' \
    "$inherited_virtual_memory_kib" >&2
  exit 2
fi

(
  if [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
    current_virtual_memory_kib="$(ulimit -v)"
    # A portable outer process guard uses a lower virtual-memory ceiling than
    # the Rust ceiling. Never try to raise that inherited hard limit: keep the
    # stricter parent bound while retaining the 12 GiB default for unbounded
    # direct Rust invocations and for the systemd resident-memory path.
    if [[ "$current_virtual_memory_kib" == "unlimited" ]] \
      || { [[ "$current_virtual_memory_kib" =~ ^[0-9]+$ ]] \
        && ((current_virtual_memory_kib > virtual_memory_kib)); }; then
      if ! ulimit -v "$virtual_memory_kib"; then
        printf '[rust-build-guard] unable to apply virtual-memory ceiling on %s\n' "$host_platform" >&2
        exit 1
      fi
    fi
  else
    printf '[rust-build-guard] virtual-memory ceiling unavailable on %s; retaining serialized jobs and reduced compiler profile\n' "$host_platform" >&2
  fi
  # The large slskr binary can make LLVM retain several gigabytes of debug
  # metadata and incremental state even with one rustc process. Keep those
  # defaults disabled inside the guard so a build stays below the repository
  # ceiling instead of exhausting the host while LLVM is linking. The
  # production daemon is also a large single crate; a high codegen-unit count
  # keeps rustc's per-unit LLVM working set bounded during metadata checks.
  export SLSKR_BUILD_GUARD_HELD=1
  exec "${guarded_command[@]}"
)
