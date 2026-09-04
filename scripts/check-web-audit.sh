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
audit_report_is_valid() {
  jq -e 'type == "object" and (.metadata | type == "object") and (.vulnerabilities | type == "object")' \
    <<<"$1" >/dev/null 2>&1
}

for audit_attempt in 1 2 3; do
  report="$(
    npm_config_fetch_retries=2 \
      npm_config_fetch_retry_mintimeout=1000 \
      npm_config_fetch_retry_maxtimeout=5000 \
      npm_config_fetch_timeout=30000 \
      npm --prefix "$package_dir" audit --json 2>/dev/null || true
  )"
  if audit_report_is_valid "$report"; then
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

if ! audit_report_is_valid "$report"; then
  # The npm registry audit endpoint is an external service and has returned
  # transient non-report JSON during otherwise healthy installs. npm's cache
  # contains the package graph and, when its advisory metadata is available,
  # can still produce the same report without another network request.
  offline_report="$(
    npm_config_offline=true \
      npm --prefix "$package_dir" audit --json 2>/dev/null || true
  )"
  if audit_report_is_valid "$offline_report"; then
    report="$offline_report"
    echo "${package_dir} npm audit registry unavailable; using the validated offline audit cache." >&2
  else
    echo "${package_dir} npm audit did not return a vulnerability report after three attempts." >&2
    echo "${audit_error:-registry returned no usable audit response}" >&2
    exit 1
  fi
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
