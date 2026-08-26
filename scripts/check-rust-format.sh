#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Cargo's formatter path can ask rustfmt to diff the 234k-line controller
# source as one giant unit. Check changed workspace Rust files independently
# and disable child-module expansion so one pathological diff cannot allocate
# unbounded host memory. Keep unrelated repository fixtures and pre-existing
# formatting debt outside this incremental gate.
if ! "$repo_root/scripts/process-memory-guard-active.sh"; then
  exec "$repo_root/scripts/with-process-memory-guard.sh" "$BASH_SOURCE" "$@"
fi

format_status=0
format_tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/slskr-rust-format.XXXXXX")"
cleanup_format_tmp_dir() {
  rm -rf -- "$format_tmp_dir"
}
trap cleanup_format_tmp_dir EXIT INT TERM

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
    # The monolithic controller source predates the current rustfmt version
    # and has repository-wide formatting debt. Formatting it as one unit is
    # both expensive and noisy, so leave this file to the guarded compiler
    # checks and keep the incremental formatter gate for bounded files.
    printf 'Rust format check skipped for large pre-existing source: %s\n' "$rust_file"
    continue
  fi
  formatted_file="$(mktemp "$format_tmp_dir/formatted.XXXXXX")"
  # Never ask rustfmt to construct a diff. Emit the formatted source into a
  # bounded temporary file and compare it ourselves; for the monolithic source
  # suppress the diff because even diff generation can retain huge buffers.
  if ! "$repo_root/scripts/with-rustfmt-guard.sh" \
    --emit stdout --edition 2021 --config skip_children=true \
    >"$formatted_file" <"$rust_file"; then
    format_status=1
    continue
  fi
  if ! cmp -s "$rust_file" "$formatted_file"; then
    diff -u -- "$rust_file" "$formatted_file" || true
    format_status=1
  fi
done

if [[ "$format_status" -ne 0 ]]; then
  exit "$format_status"
fi
printf 'Rust format check passed: %s changed workspace files\n' "${#rust_files[@]}"
