#!/usr/bin/env bash
set -euo pipefail

# npm audit can load a large dependency graph. Bound direct invocations too.
runner_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${SLSKR_PROCESS_MEMORY_GUARD_HELD:-0}" != "1" ]]; then
  exec "$runner_repo_root/scripts/with-process-memory-guard.sh" "${BASH_SOURCE[0]}" "$@"
fi

# React Router's current advisory applies only to its RSC/server-action
# adapters. slskR uses BrowserRouter exclusively and ships no RSC runtime.
# Keep the finding visible while failing on every vulnerability outside this
# explicitly reviewed, non-applicable surface.
package_dir="${1:-web}"
report="$(npm --prefix "$package_dir" audit --json 2>/dev/null || true)"
printf '%s\n' "$report" | jq '.metadata, .vulnerabilities'

unexpected="$(printf '%s\n' "$report" | jq '[.vulnerabilities | to_entries[] | select(.key != "react-router" and .key != "react-router-dom")] | length')"
if [[ "$unexpected" != "0" ]]; then
  echo "Unexpected ${package_dir} dependency vulnerabilities detected" >&2
  exit 1
fi

if printf '%s\n' "$report" | jq -e '.vulnerabilities["react-router"] or .vulnerabilities["react-router-dom"]' >/dev/null; then
  echo "Reviewed react-router RSC advisory remains visible; slskR has no RSC/server-action surface." >&2
fi
