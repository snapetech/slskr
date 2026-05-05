#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

if ! rg -n -U 'export const getToken = \(\) =>\s*getSessionStorageItem\(tokenKey\)' web/src/lib/token.js >/dev/null; then
  printf 'browser token persistence check failed: web token reader must use sessionStorage only\n' >&2
  status=1
fi

if rg -n -g '!*.test.js' -g '!*.test.jsx' -g '!*.test.ts' -g '!*.test.tsx' 'setToken\(\s*(window\.)?localStorage|localStorage\.setItem\([^)]*slskr-token|rememberMe\s*\?\s*localStorage' web/src dashboard/src; then
  printf 'browser token persistence check failed: API token persistence sink matched above\n' >&2
  status=1
fi

if ! rg -n 'sessionStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token|window\.sessionStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token' web/src >/dev/null; then
  printf 'browser token persistence check failed: ListenBrainz token must use sessionStorage\n' >&2
  status=1
fi

if rg -n -g '!*.test.js' -g '!*.test.jsx' -g '!*.test.ts' -g '!*.test.tsx' 'localStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token|window\.localStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token' web/src; then
  printf 'browser token persistence check failed: ListenBrainz token localStorage sink matched above\n' >&2
  status=1
fi

if ! rg -n 'useSessionStorage<string \| null>\('\''apiKey'\''' dashboard/src/context/ApiContext.tsx >/dev/null; then
  printf 'browser token persistence check failed: dashboard apiKey must use session storage\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'browser token persistence check passed\n'
