#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

expected_fallback_limit="262144"
if [[ "$(uname -s)" == "Darwin" ]]; then
  expected_fallback_limit="unlimited"
fi

guard=scripts/with-process-memory-guard.sh

if SLSKR_PROCESS_MEMORY_MAX_KIB=4194305 \
  "$guard" bash -c 'exit 0' >/dev/null 2>&1; then
  printf 'Process memory guard test failed: over-limit memory was accepted\n' >&2
  exit 1
fi

fallback_limit="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
  SLSKR_PROCESS_MEMORY_MAX_KIB=262144 \
    "$guard" bash -c 'ulimit -v'
)"
if [[ "$fallback_limit" != "$expected_fallback_limit" ]]; then
  printf 'Process memory guard test failed: fallback limit was %s\n' "$fallback_limit" >&2
  exit 1
fi

spoofed_marker_limit="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
  SLSKR_PROCESS_MEMORY_MAX_KIB=262144 \
  SLSKR_PROCESS_MEMORY_GUARD_HELD=1 \
    "$guard" bash -c 'ulimit -v'
)"
if [[ "$spoofed_marker_limit" != "$expected_fallback_limit" ]]; then
  printf 'Process memory guard test failed: externally supplied nesting marker bypassed the limit: %s\n' "$spoofed_marker_limit" >&2
  exit 1
fi

fallback_marker="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
    "$guard" bash -c 'printf "%s" "$SLSKR_PROCESS_MEMORY_GUARD_HELD"'
)"
if [[ "$fallback_marker" != "1" ]]; then
  printf 'Process memory guard test failed: fallback nesting marker was %s\n' "$fallback_marker" >&2
  exit 1
fi

node_options="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
    "$guard" node -e 'process.stdout.write(process.env.NODE_OPTIONS || "")'
)"
if [[ "$node_options" != *--max-old-space-size=1024* ]]; then
  printf 'Process memory guard test failed: Node heap cap was not installed\n' >&2
  exit 1
fi

working_directory="$(
  "$guard" node -e 'process.stdout.write(process.cwd())'
)"
if [[ "$working_directory" != "$repo_root" ]]; then
  printf 'Process memory guard test failed: working directory was %s\n' "$working_directory" >&2
  exit 1
fi

forwarded_environment="$(
  SLSKDN_BINARY_PATH=/tmp/frozen-slskdn \
  SLSKR_CROSS_CLIENT_INDEX=bounded \
    "$guard" bash -c 'printf "%s|%s|%s" "$SLSKDN_BINARY_PATH" "$SLSKR_CROSS_CLIENT_INDEX" "$SLSKR_PROCESS_MEMORY_GUARD_HELD"'
)"
if [[ "$forwarded_environment" != "/tmp/frozen-slskdn|bounded|1" ]]; then
  printf 'Process memory guard test failed: caller environment was not forwarded: %s\n' "$forwarded_environment" >&2
  exit 1
fi

# A verified fallback may be reused, but a tighter nested request must apply.
nested_fallback_limit="$(
  SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD=1 \
  SLSKR_PROCESS_MEMORY_MAX_KIB=262144 \
    "$guard" bash -c 'SLSKR_PROCESS_MEMORY_MAX_KIB=131072 "$1" bash -c "ulimit -v"' _ "$guard"
)"
expected_nested_limit=131072
if [[ "$(uname -s)" == "Darwin" ]]; then
  expected_nested_limit=unlimited
fi
if [[ "$nested_fallback_limit" != "$expected_nested_limit" ]]; then
  printf 'Process memory guard test failed: tighter nested limit was %s\n' "$nested_fallback_limit" >&2
  exit 1
fi

# Linux cgroup nesting must retain real memory/swap limits without RLIMIT_AS.
if [[ "$(uname -s)" == "Linux" ]] \
  && command -v systemd-run >/dev/null 2>&1 \
  && systemctl --user show-environment >/dev/null 2>&1 \
  && [[ "${SLSKR_PROCESS_MEMORY_GUARD_DISABLE_SYSTEMD:-0}" != "1" ]]; then
  nested_cgroup_limit="$(
    "$guard" bash -c '"$1" bash -c "ulimit -v"' _ "$guard"
  )"
  if [[ "$nested_cgroup_limit" != "unlimited" ]]; then
    printf 'Process memory guard test failed: verified cgroup gained virtual limit %s\n' "$nested_cgroup_limit" >&2
    exit 1
  fi
fi

printf 'Process memory guard tests passed\n'
