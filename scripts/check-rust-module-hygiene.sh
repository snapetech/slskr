#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if rg -n '^#!\[allow\([^]]*dead_code' crates --glob '!target/**'; then
  printf 'rust module hygiene failed: crate/module-level dead_code allowances are not allowed\n' >&2
  status=1
fi

for file in crates/slskr/src/main.rs crates/slskr/src/webhooks.rs crates/slskr/src/routing.rs; do
  if rg -n '^#!\[allow\([^]]*dead_code' "$file" >/dev/null; then
    printf 'rust module hygiene failed: broad dead_code allow remains in %s\n' "$file" >&2
    status=1
  fi
done

if ! rg -n '^\| BUG-025 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'rust module hygiene failed: BUG-025 must stay verified in council ledger\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rust module hygiene check passed\n'
