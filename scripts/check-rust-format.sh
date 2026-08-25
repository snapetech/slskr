#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Cargo's formatter path can ask rustfmt to diff the 234k-line controller
# source as one giant unit. Check each tracked Rust file independently and
# disable child-module expansion so one pathological diff cannot allocate
# unbounded host memory.
if [[ "${SLSKR_RUST_FORMAT_GUARD_HELD:-0}" != "1" ]]; then
  exec "$repo_root/scripts/with-process-memory-guard.sh" env \
    SLSKR_RUST_FORMAT_GUARD_HELD=1 "$BASH_SOURCE" "$@"
fi

mapfile -t rust_files < <(rg --files -g '*.rs' -g '!target/**' | sort)
if [[ "${#rust_files[@]}" -eq 0 ]]; then
  printf 'Rust format check failed: no Rust sources were found\n' >&2
  exit 1
fi

for rust_file in "${rust_files[@]}"; do
  rustfmt --check --edition 2021 --config skip_children=true "$rust_file"
done

printf 'Rust format check passed: %s files\n' "${#rust_files[@]}"
