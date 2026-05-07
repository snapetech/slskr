#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

report="${COUNCIL_OUT_DIR:-.council}/latest-candidate-counts.md"
backlog="docs/dev/bug-council-active-backlog.md"
failed=0

fail() {
  printf 'FAIL %s\n' "$1" >&2
  failed=1
}

pass() {
  printf 'PASS %s\n' "$1"
}

if [[ ! -f "$report" ]]; then
  mkdir -p "$(dirname "$report")"
  scripts/run-council-scan.sh >"$report"
fi

if [[ ! -f "$backlog" ]]; then
  fail "active backlog is missing: $backlog"
  exit 1
fi

if rg -n '\| `[^`]+` \| [0-9]+ \| Untriaged \|' "$backlog" >/tmp/slskr-active-backlog-untriaged.$$ 2>/dev/null; then
  fail "active backlog contains untriaged sections"
  sed 's/^/  /' /tmp/slskr-active-backlog-untriaged.$$ >&2
else
  pass "active backlog has no untriaged sections"
fi
rm -f /tmp/slskr-active-backlog-untriaged.$$

awk -F'|' '
  $2 ~ /Candidate Class/ || $2 ~ /^ --- / {
    next
  }

  $2 ~ /[^[:space:]]/ && $3 ~ /[0-9]/ {
    section = $2
    count = $3
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", section)
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", count)
    print section "\t" count
  }
' "$report" >/tmp/slskr-active-backlog-counts.$$

while IFS=$'\t' read -r section count; do
  if rg -n --fixed-strings "| \`$section\` | $count |" "$backlog" >/dev/null; then
    pass "active backlog tracks '$section' count $count"
  else
    fail "active backlog missing or stale for '$section' count $count"
  fi
done </tmp/slskr-active-backlog-counts.$$

rm -f /tmp/slskr-active-backlog-counts.$$

if [[ "$failed" -ne 0 ]]; then
  printf '\nActive backlog check failed. Run scripts/run-council-scan.sh, then update %s.\n' "$backlog" >&2
  exit 1
fi

printf '\nAll active backlog checks passed.\n'
