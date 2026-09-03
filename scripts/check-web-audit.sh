#!/usr/bin/env bash
set -euo pipefail

# npm audit can load a large dependency graph. Bound direct invocations too.
runner_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if ! "$runner_repo_root/scripts/process-memory-guard-active.sh"; then
  exec "$runner_repo_root/scripts/with-process-memory-guard.sh" "${BASH_SOURCE[0]}" "$@"
fi

# React Router's current advisory applies only to its RSC/server-action
# adapters. slskR uses BrowserRouter exclusively and ships no RSC runtime.
# Keep the finding visible while failing on every vulnerability outside this
# explicitly reviewed, non-applicable surface.
package_dir="${1:-web}"
report=""
audit_error=""
for audit_attempt in 1 2 3; do
  report="$(
    npm_config_fetch_retries=2 \
      npm_config_fetch_retry_mintimeout=1000 \
      npm_config_fetch_retry_maxtimeout=5000 \
      npm_config_fetch_timeout=30000 \
      npm --prefix "$package_dir" audit --json 2>/dev/null || true
  )"
  if jq -e 'type == "object" and (.metadata | type == "object") and (.vulnerabilities | type == "object")' \
    <<<"$report" >/dev/null 2>&1; then
    break
  fi
  audit_error="$(
    jq -r '
      if type == "object" then
        (.error.detail // .error.summary // .message // "invalid npm audit response")
      else
        "invalid npm audit response"
      end
    ' <<<"$report" 2>/dev/null || true
  )"
  if (( audit_attempt < 3 )); then
    sleep "$audit_attempt"
  fi
done

if ! jq -e 'type == "object" and (.metadata | type == "object") and (.vulnerabilities | type == "object")' \
  <<<"$report" >/dev/null 2>&1; then
  echo "${package_dir} npm audit did not return a vulnerability report after three attempts." >&2
  echo "${audit_error:-registry returned no usable audit response}" >&2
  exit 1
fi

printf '%s\n' "$report" | jq '.metadata, .vulnerabilities'

unexpected="$(printf '%s\n' "$report" | jq '[.vulnerabilities | to_entries[] | select(.key != "react-router" and .key != "react-router-dom")] | length')"
if [[ "$unexpected" != "0" ]]; then
  echo "Unexpected ${package_dir} dependency vulnerabilities detected" >&2
  exit 1
fi

if printf '%s\n' "$report" | jq -e '.vulnerabilities["react-router"] or .vulnerabilities["react-router-dom"]' >/dev/null; then
  echo "Reviewed react-router RSC advisory remains visible; slskR has no RSC/server-action surface." >&2
fi
