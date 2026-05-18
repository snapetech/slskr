#!/usr/bin/env bash
set -euo pipefail

for pkgbuild in "$@"; do
  grep -q '^sha256sums=' "$pkgbuild"
  if grep -q "'SKIP'.*'SKIP'.*'SKIP'.*'SKIP'.*'SKIP'" "$pkgbuild"; then
    echo "$pkgbuild still has placeholder-only hashes" >&2
    exit 1
  fi
done
