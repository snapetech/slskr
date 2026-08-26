#!/usr/bin/env bash
set -euo pipefail

# A nesting marker is valid only when the current process also carries the
# resource boundary that the process guard promises. This prevents a caller
# supplied environment variable from skipping the outer guard.
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_HELD:-0}" != "1" ]]; then
  exit 1
fi

if [[ "$(uname -s 2>/dev/null || printf 'unknown')" == "Darwin" ]]; then
  # Darwin has no settable `ulimit -v` in the supported shell path.
  exit 0
fi

current_virtual_memory_kib="$(ulimit -v)"
if [[ "$current_virtual_memory_kib" =~ ^[0-9]+$ ]] \
  && ((current_virtual_memory_kib <= 4194304)); then
  exit 0
fi

if grep -Eq '/slskr-process-memory-guard-[^/]+\.service' /proc/self/cgroup 2>/dev/null; then
  exit 0
fi

exit 1
