#!/usr/bin/env bash
set -euo pipefail

repo_root="${SLSKR_COUNCIL_FRESHNESS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
expected_date="${SLSKR_COUNCIL_EXPECTED_DATE:-$(date +%Y-%m-%d)}"
failed=0

fail() {
  printf 'council freshness check failed: %s\n' "$1" >&2
  failed=1
}

if [[ -n "${SLSKR_COUNCIL_EXPECTED_DIGEST:-}" ]]; then
  expected_digest="$SLSKR_COUNCIL_EXPECTED_DIGEST"
else
  if ! expected_digest="$(python3 "$repo_root/scripts/council-source-digest.py" 2>/dev/null)"; then
    fail "unable to determine the source digest for $repo_root"
    expected_digest=""
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

  if [[ -n "$expected_digest" ]] && rg -n "^Source digest: ${expected_digest}$" "$file" >/dev/null; then
    printf 'PASS source digest: %s (%s)\n' "$expected_digest" "${file#"$repo_root/"}"
  elif [[ -n "$expected_digest" ]]; then
    fail "${file#"$repo_root/"} is not stamped with source digest $expected_digest"
  fi
done

if [[ -n "$expected_digest" ]] && ! [[ "$expected_digest" =~ ^[0-9a-f]{64}$ ]]; then
  fail "source digest is not a SHA-256 value: $expected_digest"
fi

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

printf 'council freshness check passed\n'
