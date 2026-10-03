#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

pattern='innerHTML[[:space:]]*=|dangerouslySetInnerHTML|document\.write[[:space:]]*\(|\beval[[:space:]]*\(|new[[:space:]]+Function[[:space:]]*\('

if rg -n --pcre2 \
  --glob '*.{js,jsx,ts,tsx}' \
  --glob '!**/*.test.*' \
  --glob '!**/*.spec.*' \
  --glob '!**/__tests__/**' \
  "$pattern" web/src dashboard/src client-ts/src; then
  printf 'browser injection sink check failed: use React text rendering or a reviewed safe API\n' >&2
  exit 1
fi

printf 'browser injection sink check passed\n'
