#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

report="${COUNCIL_OUT_DIR:-.council}/latest-candidate-counts.md"
inventory="docs/dev/council-scan-inventory.md"
backlog="docs/dev/bug-council-active-backlog.md"
status=0

if [[ ! -f "$report" ]]; then
  mkdir -p "$(dirname "$report")"
  scripts/run-council-scan.sh >"$report"
fi

fail() {
  printf 'FAIL %s\n' "$1" >&2
  status=1
}

pass() {
  printf 'PASS %s\n' "$1"
}

while IFS=$'\t' read -r section count; do
  if ! rg -n --fixed-strings "| $section | Fixed |" "$inventory" >/dev/null; then
    fail "inventory does not mark '$section' fixed"
    continue
  fi

  if rg -n --fixed-strings "| \`$section\` | $count | Guarded |" "$backlog" >/dev/null; then
    pass "inventory closure tracks '$section' as guarded at count $count"
  else
    fail "active backlog does not guard '$section' at count $count"
  fi
done < <(
  awk -F'|' '
    $2 ~ /Candidate Class/ || $2 ~ /^ --- / { next }
    $2 ~ /[^[:space:]]/ && $3 ~ /[0-9]/ {
      section = $2
      count = $3
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", section)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", count)
      print section "\t" count
    }
  ' "$report"
)

if [[ "$status" -ne 0 ]]; then
  printf '\nCouncil inventory closure check failed. Update %s and %s from the fresh scan.\n' "$inventory" "$backlog" >&2
  exit "$status"
fi

printf '\nCouncil inventory closure check passed.\n'
