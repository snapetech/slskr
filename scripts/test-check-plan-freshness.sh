#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/slskr-plan-freshness.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

mkdir -p "$tmp_root/docs/dev"
cp "$repo_root/PLAN.md" "$tmp_root/PLAN.md"
cp "$repo_root/REMEDIATION.md" "$tmp_root/REMEDIATION.md"
cp "$repo_root/docs/dev/refactoring-efficiency-plan.md" "$tmp_root/docs/dev/refactoring-efficiency-plan.md"
cp "$repo_root/docs/dev/refactor-audit-20260915.md" "$tmp_root/docs/dev/refactor-audit-20260915.md"

SLSKR_PLAN_FRESHNESS_ROOT="$tmp_root" \
  "$repo_root/scripts/check-plan-freshness.sh" >/dev/null

sed -i 's/^Status: active whole-project implementation plan,/Status: stale implementation plan,/' \
  "$tmp_root/docs/dev/refactoring-efficiency-plan.md"
if SLSKR_PLAN_FRESHNESS_ROOT="$tmp_root" \
  "$repo_root/scripts/check-plan-freshness.sh" >/dev/null 2>&1; then
  printf 'plan freshness test failed: stale active-plan status was not rejected\n' >&2
  exit 1
fi

printf 'plan freshness tests passed\n'
