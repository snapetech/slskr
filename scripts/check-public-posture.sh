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
scan_raw="$(mktemp)"
scan_exact_filtered="$(mktemp)"
scan_filtered="$(mktemp)"
trap 'rm -f "$scan_raw" "$scan_exact_filtered" "$scan_filtered"' EXIT

rg_allow_no_matches() {
  local output_file="$1"
  shift
  if rg "$@" >"$output_file"; then
    return 0
  else
    local scan_status=$?
    if [[ "$scan_status" -eq 1 ]]; then
      return 0
    fi
    printf 'public posture scan failed (rg exit %s)\n' "$scan_status" >&2
    return "$scan_status"
  fi
}

for pattern in "${patterns[@]}"; do
  # Preserve the frozen slskdN default user-description value only where it
  # is required for controller compatibility and differential fixtures.
  rg_allow_no_matches "$scan_raw" -n -i -F "$pattern" README.md PLAN.md COMPLIANCE.md NOTICE Cargo.toml crates client-go client-python client-ts web docs k8s .github
  rg_allow_no_matches "$scan_exact_filtered" -v -F 'A slskdN user. Unofficial fork of slskd: https://github.com/snapetech/slskdn' "$scan_raw"
  rg_allow_no_matches "$scan_filtered" -v -i 'do not|should not|must not|unless|avoid|remove casual|presenting the repository|not copied|not copy|not import|not say|prohibited|forbidden|current web ui as the reference implementation|based on error type' "$scan_exact_filtered"
  matches="$(<"$scan_filtered")"
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
