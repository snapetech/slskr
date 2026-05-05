#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
policy="docs/dev/rust-dependency-hygiene.md"
status=0

if [[ ! -f "$policy" ]]; then
  printf 'rust dependency hygiene failed: missing %s\n' "$policy" >&2
  exit 1
fi

duplicate_roots="$(
  cargo tree -d -p slskr |
    awk '/^[[:alnum:]_-]+ v[0-9]/{print $1}' |
    sort -u
)"

allowed_roots="$(printf '%s\n' getrandom hashbrown thiserror thiserror-impl | sort -u)"

unexpected="$(comm -13 <(printf '%s\n' "$allowed_roots") <(printf '%s\n' "$duplicate_roots"))"
missing="$(comm -23 <(printf '%s\n' "$allowed_roots") <(printf '%s\n' "$duplicate_roots"))"

if [[ -n "$unexpected" ]]; then
  printf 'rust dependency hygiene failed: unexpected duplicate roots:\n%s\n' "$unexpected" >&2
  status=1
fi

if [[ -n "$missing" ]]; then
  printf 'rust dependency hygiene note: duplicate roots resolved; update %s and BUG-021:\n%s\n' "$policy" "$missing"
fi

for root in getrandom hashbrown thiserror thiserror-impl; do
  if ! rg -n -F "| \`$root\` |" "$policy" >/dev/null; then
    printf 'rust dependency hygiene failed: %s missing from %s\n' "$root" "$policy" >&2
    status=1
  fi
done

if ! rg -n '^\| BUG-021 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'rust dependency hygiene failed: BUG-021 must stay verified in council ledger\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rust dependency hygiene check passed\n'
