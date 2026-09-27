#!/usr/bin/env bash
set -euo pipefail

repo_root="${SLSKR_COUNCIL_FRESHNESS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
expected_date="${SLSKR_COUNCIL_EXPECTED_DATE:-$(date +%Y-%m-%d)}"
failed=0

fail() {
  printf 'council freshness check failed: %s\n' "$1" >&2
  failed=1
}

if [[ -n "${SLSKR_COUNCIL_EXPECTED_COMMIT:-}" ]]; then
  expected_commit="$SLSKR_COUNCIL_EXPECTED_COMMIT"
else
  if ! expected_commit="$(git -C "$repo_root" rev-parse HEAD 2>/dev/null)"; then
    fail "unable to determine the source commit for $repo_root"
    expected_commit=""
  fi
fi

files=(
  "$repo_root/.council/latest-candidate-counts.md"
  "$repo_root/docs/dev/council-scan-inventory.md"
  "$repo_root/docs/dev/bug-council-active-backlog.md"
)

for file in "${files[@]}"; do
  if [[ ! -f "$file" ]]; then
    fail "required council record is missing: ${file#"$repo_root/"}"
    continue
  fi

  if rg -n "^Scan date: ${expected_date}$" "$file" >/dev/null; then
    printf 'PASS scan date: %s (%s)\n' "$expected_date" "${file#"$repo_root/"}"
  else
    fail "${file#"$repo_root/"} is not stamped with scan date $expected_date"
  fi

  if [[ -n "$expected_commit" ]] && rg -n "^Source commit: ${expected_commit}$" "$file" >/dev/null; then
    printf 'PASS source commit: %s (%s)\n' "$expected_commit" "${file#"$repo_root/"}"
  elif [[ -n "$expected_commit" ]]; then
    fail "${file#"$repo_root/"} is not stamped with source commit $expected_commit"
  fi
done

if [[ -n "$expected_commit" ]] && ! [[ "$expected_commit" =~ ^[0-9a-f]{40}$ ]]; then
  fail "source commit is not a full hexadecimal Git object id: $expected_commit"
fi

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

printf 'council freshness check passed\n'
