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

requested_memory_kib="${1:-4194304}"
if [[ ! "$requested_memory_kib" =~ ^[1-9][0-9]{0,7}$ ]] \
  || ((requested_memory_kib > 4194304)); then
  exit 1
fi
current_virtual_memory_kib="$(ulimit -v)"
if [[ "$current_virtual_memory_kib" =~ ^[0-9]+$ ]] \
  && ((current_virtual_memory_kib <= requested_memory_kib)); then
  exit 0
fi

# Check the actual cgroup limits, not just its name or the nesting marker.
if [[ -r /proc/self/cgroup ]]; then
  while IFS=: read -r hierarchy controllers guard_cgroup_path; do
    if [[ "$hierarchy" != "0" || -n "$controllers" ]]; then
      continue
    fi
    case "$guard_cgroup_path" in
      */slskr-process-memory-guard-*.service)
        guard_cgroup_directory="/sys/fs/cgroup$guard_cgroup_path"
        if [[ ! -r "$guard_cgroup_directory/memory.max" \
          || ! -r "$guard_cgroup_directory/memory.swap.max" ]]; then
          continue
        fi
        read -r inherited_memory_bytes < "$guard_cgroup_directory/memory.max"
        read -r inherited_swap_bytes < "$guard_cgroup_directory/memory.swap.max"
        if [[ "$inherited_memory_bytes" =~ ^[0-9]{1,12}$ \
          && "$inherited_swap_bytes" == "0" ]] \
          && ((inherited_memory_bytes <= requested_memory_kib * 1024)); then
          exit 0
        fi
        ;;
    esac
  done < /proc/self/cgroup
fi

exit 1
