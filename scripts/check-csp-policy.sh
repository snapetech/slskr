#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0
csp_scan_paths=(crates/slskr/src crates/slskr-web/static)

if [[ -f web/build/index.html ]]; then
  csp_scan_paths+=(web/build/index.html)
fi

if rg -n "Content-Security-Policy: .*'unsafe-inline'|script-src .*'unsafe-inline'|style-src .*'unsafe-inline'" "${csp_scan_paths[@]}"; then
  printf 'csp policy failed: broad unsafe-inline CSP allowance is present in served source/build files\n' >&2
  status=1
fi

if rg -n "<script type=\"module\">|<script>" crates/slskr-web/static/index.html; then
  printf 'csp policy failed: Rust WASM shell must use an external bootstrap module\n' >&2
  status=1
fi

if ! rg -q "script-src 'self';" crates/slskr/src/main.rs; then
  printf 'csp policy failed: non-WASM static policy must keep script-src self-only\n' >&2
  status=1
fi

if ! rg -q "script-src 'self' 'wasm-unsafe-eval'" crates/slskr/src/main.rs; then
  printf 'csp policy failed: Rust WASM shell exception must be explicit and scoped\n' >&2
  status=1
fi

wasm_exception_count="$(rg -n "script-src 'self' 'wasm-unsafe-eval'" "${csp_scan_paths[@]}" | awk '!/assert!/ { count++ } END { print count + 0 }')"
if [[ "$wasm_exception_count" != "1" ]]; then
  printf 'csp policy failed: wasm-unsafe-eval should appear only in the scoped served policy\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'csp policy passed\n'
