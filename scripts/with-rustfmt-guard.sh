#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  printf 'usage: %s <rustfmt arguments...>\n' "${BASH_SOURCE[0]}" >&2
  exit 2
fi

# Direct rustfmt invocations do not pass through Cargo's build guard. Route
# them through the 4 GiB process-memory/no-swap ceiling as well.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$repo_root/scripts/with-process-memory-guard.sh" rustfmt "$@"
