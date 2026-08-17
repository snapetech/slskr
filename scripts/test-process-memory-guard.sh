#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"
cd "$repo_root"

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
if [[ "$fallback_limit" != "262144" ]]; then
  printf 'Process memory guard test failed: fallback limit was %s\n' "$fallback_limit" >&2
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

printf 'Process memory guard tests passed\n'
