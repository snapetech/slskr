#!/usr/bin/env bash
set -euo pipefail

repo_root="${SLSKR_PLAN_FRESHNESS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$repo_root"

status=0

required_files=(
  PLAN.md
  REMEDIATION.md
  docs/dev/refactoring-efficiency-plan.md
  docs/dev/refactor-audit-20260915.md
)
for path in "${required_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    printf 'plan freshness check failed: required file is missing: %s\n' "$path" >&2
    status=1
  fi
done

if [[ -f PLAN.md ]] && ! rg -n -F 'docs/dev/refactoring-efficiency-plan.md' PLAN.md >/dev/null; then
  printf 'plan freshness check failed: PLAN.md does not point to the active refactor plan\n' >&2
  status=1
fi

if [[ -f PLAN.md ]] && ! rg -n -F 'docs/dev/refactor-audit-20260915.md' PLAN.md >/dev/null; then
  printf 'plan freshness check failed: PLAN.md does not point to the active refactor evidence ledger\n' >&2
  status=1
fi

if [[ -f REMEDIATION.md ]] && ! rg -n '^# slskr Historical Remediation Baseline$' REMEDIATION.md >/dev/null; then
  printf 'plan freshness check failed: REMEDIATION.md is not marked as historical\n' >&2
  status=1
fi

if [[ -f REMEDIATION.md ]] && rg -n -F 'single source of truth for what is real' REMEDIATION.md >/dev/null; then
  printf 'plan freshness check failed: REMEDIATION.md still claims current single-source ownership\n' >&2
  status=1
fi

if [[ -f REMEDIATION.md ]] && ! rg -n '^## 0\. Historical snapshot \([0-9]{4}-[0-9]{2}-[0-9]{2}\)$' REMEDIATION.md >/dev/null; then
  printf 'plan freshness check failed: REMEDIATION.md has no dated historical snapshot\n' >&2
  status=1
fi

if [[ -f docs/dev/refactoring-efficiency-plan.md ]] && ! rg -n '^Status: active whole-project implementation plan,' docs/dev/refactoring-efficiency-plan.md >/dev/null; then
  printf 'plan freshness check failed: active refactor plan has no current status marker\n' >&2
  status=1
fi

if [[ -f docs/dev/refactor-audit-20260915.md ]] && ! rg -n '^\| RF-073 \|' docs/dev/refactor-audit-20260915.md >/dev/null; then
  printf 'plan freshness check failed: refactor evidence ledger is missing its current RF range\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'plan freshness check passed\n'
