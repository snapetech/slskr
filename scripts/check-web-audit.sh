#!/usr/bin/env bash
set -euo pipefail

# React Router's current advisory applies only to its RSC/server-action
# adapters. slskR uses BrowserRouter exclusively and ships no RSC runtime.
# Keep the finding visible while failing on every vulnerability outside this
# explicitly reviewed, non-applicable surface.
report="$(npm --prefix web audit --json 2>/dev/null || true)"
printf '%s\n' "$report" | jq '.metadata, .vulnerabilities'

unexpected="$(printf '%s\n' "$report" | jq '[.vulnerabilities | to_entries[] | select(.key != "react-router" and .key != "react-router-dom")] | length')"
if [[ "$unexpected" != "0" ]]; then
  echo "Unexpected web dependency vulnerabilities detected" >&2
  exit 1
fi

if printf '%s\n' "$report" | jq -e '.vulnerabilities["react-router"] or .vulnerabilities["react-router-dom"]' >/dev/null; then
  echo "Reviewed react-router RSC advisory remains visible; slskR has no RSC/server-action surface." >&2
fi
