#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/slskr-council-freshness.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

mkdir -p "$tmp_root/.council" "$tmp_root/docs/dev"
cp "$repo_root/.council/latest-candidate-counts.md" "$tmp_root/.council/latest-candidate-counts.md"
cp "$repo_root/docs/dev/council-scan-inventory.md" "$tmp_root/docs/dev/council-scan-inventory.md"
cp "$repo_root/docs/dev/bug-council-active-backlog.md" "$tmp_root/docs/dev/bug-council-active-backlog.md"

expected_date="$(rg -o '^Scan date: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$tmp_root/.council/latest-candidate-counts.md" | cut -d' ' -f3)"
expected_commit="$(rg -o '^Source commit: [0-9a-f]{40}$' "$tmp_root/.council/latest-candidate-counts.md" | cut -d' ' -f3)"

SLSKR_COUNCIL_FRESHNESS_ROOT="$tmp_root" \
SLSKR_COUNCIL_EXPECTED_DATE="$expected_date" \
SLSKR_COUNCIL_EXPECTED_COMMIT="$expected_commit" \
  "$repo_root/scripts/check-council-freshness.sh" >/dev/null

sed -i "s/^Scan date: .*/Scan date: 2000-01-01/" "$tmp_root/docs/dev/council-scan-inventory.md"
if SLSKR_COUNCIL_FRESHNESS_ROOT="$tmp_root" \
  SLSKR_COUNCIL_EXPECTED_DATE="$expected_date" \
  SLSKR_COUNCIL_EXPECTED_COMMIT="$expected_commit" \
  "$repo_root/scripts/check-council-freshness.sh" >/dev/null 2>&1; then
  printf 'council freshness test failed: stale scan date was not rejected\n' >&2
  exit 1
fi

printf 'council freshness tests passed\n'
