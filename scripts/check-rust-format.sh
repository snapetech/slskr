#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Cargo's formatter path can ask rustfmt to diff the 234k-line controller
# source as one giant unit. Check changed workspace Rust files independently
# and disable child-module expansion so one pathological diff cannot allocate
# unbounded host memory. Keep unrelated repository fixtures and pre-existing
# formatting debt outside this incremental gate.
if [[ "${SLSKR_RUST_FORMAT_GUARD_HELD:-0}" != "1" ]]; then
  exec "$repo_root/scripts/with-process-memory-guard.sh" env \
    SLSKR_RUST_FORMAT_GUARD_HELD=1 "$BASH_SOURCE" "$@"
fi

format_status=0
format_base="${SLSKR_RUST_FORMAT_BASE:-}"
if [[ -n "$format_base" ]]; then
  mapfile -t rust_files < <(git diff --name-only "$format_base" HEAD -- 'crates/**/*.rs' | sort -u)
elif git diff --quiet HEAD -- && git diff --cached --quiet; then
  if git rev-parse --verify HEAD^ >/dev/null 2>&1; then
    mapfile -t rust_files < <(git diff --name-only HEAD^ HEAD -- 'crates/**/*.rs' | sort -u)
  else
    rust_files=()
  fi
else
  mapfile -t rust_files < <(
    {
      git diff --name-only HEAD -- 'crates/**/*.rs'
      git diff --cached --name-only -- 'crates/**/*.rs'
      git ls-files --others --exclude-standard -- 'crates/**/*.rs'
    } | sort -u
  )
fi
if [[ "${#rust_files[@]}" -eq 0 ]]; then
  printf 'Rust format check passed: no changed workspace Rust files\n'
  exit 0
fi

for rust_file in "${rust_files[@]}"; do
  file_bytes="$(wc -c <"$rust_file")"
  if ((file_bytes > 2000000)); then
    # Never ask rustfmt to diff a multi-megabyte source file. Its diff emitter
    # has a pathological allocation path for main.rs; parsing and formatting
    # to /dev/null still validates syntax while avoiding that diff entirely.
    if ! rustfmt --emit stdout --edition 2021 --config skip_children=true "$rust_file" >/dev/null; then
      format_status=1
    fi
  elif ! rustfmt --check --edition 2021 --config skip_children=true "$rust_file"; then
    format_status=1
  fi
done

if [[ "$format_status" -ne 0 ]]; then
  exit "$format_status"
fi
printf 'Rust format check passed: %s changed workspace files\n' "${#rust_files[@]}"
