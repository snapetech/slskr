#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The case runner may launch .NET, Rust, or browser helpers. Keep managed heaps
# bounded through their own tool settings even when a caller supplies a custom
# case runner. Rust commands use the repository Cargo configuration directly.
export DOTNET_GCHeapHardLimit="${DOTNET_GCHeapHardLimit:-1073741824}"
export COMPlus_GCHeapHardLimit="${COMPlus_GCHeapHardLimit:-1073741824}"
export NODE_OPTIONS="${NODE_OPTIONS:---max-old-space-size=1024}"

exec python3 "$repo_root/scripts/run-universal-lifecycle-matrix.py" "$@"
