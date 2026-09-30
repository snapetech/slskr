#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixtures_root="${FIXTURES_DIR:-$repo_root/test-data/slskr-test-fixtures}"

if [[ ! -f "$fixtures_root/meta/manifest.json" ]]; then
  printf 'ERROR: fixtures not found at %s (missing meta/manifest.json).\n' "$fixtures_root" >&2
  printf 'Set FIXTURES_DIR if the fixture tree is elsewhere.\n' >&2
  exit 1
fi

SLSKR_FIXTURES_ROOT="$fixtures_root" "$repo_root/scripts/check-fixture-manifest.sh"
"$fixtures_root/meta/fetch_media.sh"
