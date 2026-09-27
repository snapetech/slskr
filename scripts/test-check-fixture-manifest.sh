#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/slskr-fixture-manifest.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

cp -R "$repo_root/test-data/slskr-test-fixtures/." "$tmp_root/"
SLSKR_FIXTURES_ROOT="$tmp_root" "$repo_root/scripts/check-fixture-manifest.sh" >/dev/null

printf 'corrupt fixture\n' >> "$tmp_root/book/treasure_island_pg120.txt"
if SLSKR_FIXTURES_ROOT="$tmp_root" "$repo_root/scripts/check-fixture-manifest.sh" >/dev/null 2>&1; then
  printf 'fixture manifest test failed: corrupt fixture was not rejected\n' >&2
  exit 1
fi

printf 'fixture manifest tests passed\n'
