#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$#" -eq 0 ]]; then
  printf 'usage: %s <command> [args...]\n' "${BASH_SOURCE[0]}" >&2
  exit 2
fi

hard_memory_kib=4194304
memory_kib="${SLSKR_PROCESS_MEMORY_MAX_KIB:-$hard_memory_kib}"
tasks_max="${SLSKR_PROCESS_TASKS_MAX:-512}"
host_platform="$(uname -s 2>/dev/null || printf 'unknown')"
virtual_memory_limit_supported=1
defer_to_rust_build_guard=0
if [[ "$(basename "$1")" == "build-release-archive.sh" ]]; then
  # The archive runner builds Rust after its web build. On runners without a
  # systemd user manager, defer the virtual-memory ceiling to the nested Rust
  # guard instead of trapping Cargo inside the smaller process ceiling.
  defer_to_rust_build_guard=1
fi
if [[ "$host_platform" == "Darwin" ]]; then
  # Darwin's shell does not expose a settable RLIMIT_AS through `ulimit -v`.
  # Keep Node's managed-heap cap and the process-guard nesting marker while
  # allowing macOS release artifacts to reach their compiler commands.
  virtual_memory_limit_supported=0
fi

if [[ ! "$memory_kib" =~ ^[1-9][0-9]{0,7}$ || "$memory_kib" -gt "$hard_memory_kib" ]]; then
  printf 'SLSKR_PROCESS_MEMORY_MAX_KIB must be between 1 and %s KiB\n' "$hard_memory_kib" >&2
  exit 2
fi
if [[ ! "$tasks_max" =~ ^[1-9][0-9]{0,5}$ || "$tasks_max" -gt 512 ]]; then
  printf 'SLSKR_PROCESS_TASKS_MAX must be between 1 and 512\n' >&2
  exit 2
fi

process_guard_context_active=0
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_HELD:-0}" == "1" ]]; then
  inherited_process_virtual_memory_kib="unlimited"
  if [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
    inherited_process_virtual_memory_kib="$(ulimit -v)"
  fi
  if [[ "$inherited_process_virtual_memory_kib" =~ ^[0-9]+$ ]] \
    && ((inherited_process_virtual_memory_kib <= memory_kib)); then
    process_guard_context_active=1
  elif grep -Eq '/slskr-process-memory-guard-[^/]+\.service' /proc/self/cgroup 2>/dev/null; then
    process_guard_context_active=1
  fi
fi

# Cargo has a separate Rust build guard with a larger, serialized profile.
# Do not create the 4 GiB application/browser cgroup around that guard; doing
# so would make the Rust limit ineffective and can kill rustfmt or rustc even
# when the host has ample available memory.
if [[ "$(basename "$1")" == "with-build-guard.sh" \
  && "${2:-}" == "cargo" \
  && "$process_guard_context_active" -eq 0 ]]; then
  exec "$@"
fi

# Keep Node's managed heap below the process cap. Chromium's native mappings
# remain bounded by the cgroup or virtual-memory ceiling below.
case "$(basename "$1")" in
  node|nodejs|npm|npx|vite|vitest|playwright)
    if [[ "${NODE_OPTIONS:-}" != *--max-old-space-size=* ]]; then
      export NODE_OPTIONS="${NODE_OPTIONS:-} --max-old-space-size=1024"
    fi
    ;;
esac

# A user systemd manager gives a reliable resident-memory ceiling for Chromium
# and its renderer processes. The environment switch exists only for the
# fallback regression test; production callers use systemd when available.
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD:-0}" != "1" ]] \
  && [[ "$process_guard_context_active" -eq 0 ]] \
  && command -v systemd-run >/dev/null 2>&1 \
  && command -v systemctl >/dev/null 2>&1 \
  && systemctl --user show-environment >/dev/null 2>&1; then
  unit_name="slskr-process-memory-guard-${BASHPID}-${RANDOM}.service"
  environment_file=""
  cleanup_unit() {
    local status="$?"
    trap - EXIT INT TERM
    systemctl --user stop "$unit_name" >/dev/null 2>&1 || true
    if [[ -n "$environment_file" ]]; then
      rm -f -- "$environment_file"
    fi
    exit "$status"
  }
  trap cleanup_unit EXIT INT TERM
  environment_file="$(mktemp "${TMPDIR:-/tmp}/slskr-process-memory-guard-environment.XXXXXX")"
  chmod 600 "$environment_file"
  while IFS= read -r -d '' environment_entry; do
    case "$environment_entry" in
      SLSKR_PROCESS_MEMORY_GUARD_HELD=*)
        continue
        ;;
    esac
    environment_name="${environment_entry%%=*}"
    environment_value="${environment_entry#*=}"
    printf -v escaped_environment_value '%q' "$environment_value"
    printf '%s=%s\n' "$environment_name" "$escaped_environment_value" >> "$environment_file"
  done < <(env -0)
  systemd-run --user --wait --pipe --collect \
    --unit="$unit_name" \
    --working-directory="$repo_root" \
    --property="MemoryMax=${memory_kib}K" \
    --property="MemorySwapMax=0" \
    --property="TasksMax=${tasks_max}" \
    --property="EnvironmentFile=$environment_file" \
    --setenv=SLSKR_PROCESS_MEMORY_GUARD_HELD=1 \
    "$@"
  exit "$?"
fi

# Portable fallback for environments without a user systemd manager (including
# Git Bash and minimal containers). A virtual-memory ceiling is fail-closed: a
# browser that cannot operate within it exits instead of growing unbounded.
(
  if [[ "$defer_to_rust_build_guard" -eq 1 ]]; then
    printf '[process-memory-guard] deferring virtual-memory ceiling to nested Rust guard for release archive\n' >&2
  elif [[ "$virtual_memory_limit_supported" -eq 1 ]]; then
    if ! ulimit -v "$memory_kib"; then
      printf '[process-memory-guard] unable to apply virtual-memory ceiling on %s\n' "$host_platform" >&2
      exit 1
    fi
  else
    printf '[process-memory-guard] virtual-memory ceiling unavailable on %s; retaining managed-heap cap\n' "$host_platform" >&2
  fi
  export SLSKR_PROCESS_MEMORY_GUARD_HELD=1
  exec "$@"
)
