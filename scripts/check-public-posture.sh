#!/usr/bin/env bash
set -euo pipefail

patterns=(
  "fork"" of "
  "drop-in replacement"
  "replacement distribution"
  "based on another implementation"
  "inspiration"
  "reference implementation"
  "root implementation"
  "official variant"
  "official client"
  "successor"
)

status=0
for pattern in "${patterns[@]}"; do
  matches="$(
    # Preserve the frozen slskdN default user-description value only where it
    # is required for controller compatibility and differential fixtures.
    rg -n -i -F "$pattern" README.md PLAN.md COMPLIANCE.md NOTICE Cargo.toml crates client-go client-python client-ts web docs k8s .github \
      | rg -v -F 'A slskdN user. Unofficial fork of slskd: https://github.com/snapetech/slskdn' \
      | rg -v -i 'do not|should not|must not|unless|avoid|remove casual|presenting the repository|not copied|not copy|not import|not say|prohibited|forbidden|current web ui as the reference implementation|based on error type' || true
  )"
  if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches"
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  echo "public posture check failed: remove or reword the matches above" >&2
  exit "$status"
fi

echo "public posture check passed"
