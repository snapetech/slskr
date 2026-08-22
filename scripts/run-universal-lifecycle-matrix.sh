#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_HELD:-0}" != "1" ]]; then
  exec "$repo_root/scripts/with-process-memory-guard.sh" "${BASH_SOURCE[0]}" "$@"
fi

# The case runner may launch .NET, Rust, or browser helpers. Keep managed heaps
# bounded even when a caller supplies a custom case runner.
export DOTNET_GCHeapHardLimit="${DOTNET_GCHeapHardLimit:-1073741824}"
export COMPlus_GCHeapHardLimit="${COMPlus_GCHeapHardLimit:-1073741824}"
export NODE_OPTIONS="${NODE_OPTIONS:---max-old-space-size=1024}"

exec python3 "$repo_root/scripts/run-universal-lifecycle-matrix.py" "$@"
