#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_path="${SLSKR_REPRODUCIBILITY_OUTPUT:-target/reproducibility/current.json}"

python3 "$repo_root/scripts/collect-reproducibility-metadata.py" \
  --output "$output_path" \
  --validate
printf 'reproducibility metadata check passed: %s\n' "$output_path"
