#!/usr/bin/env bash
set -euo pipefail

patterns=(
  "fork of "
  "drop-in replacement"
  "replacement distribution"
  "based on "
  "official variant"
  "official client"
)

status=0
for pattern in "${patterns[@]}"; do
  if rg -n -F "$pattern" README.md Cargo.toml crates .github; then
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  echo "public posture check failed: remove or reword the matches above" >&2
  exit "$status"
fi

echo "public posture check passed"
