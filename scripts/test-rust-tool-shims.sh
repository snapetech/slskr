#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
shim_dir="$(mktemp -d "${TMPDIR:-/tmp}/slskr-rust-shims.XXXXXX")"
cleanup() {
  rm -rf -- "$shim_dir"
}
trap cleanup EXIT INT TERM

ln -s "$repo_root/scripts/rust-tool-shim.sh" "$shim_dir/rustfmt"
ln -s "$repo_root/scripts/rust-tool-shim.sh" "$shim_dir/cargo"

set +e
rustfmt_result="$(
  cd "$repo_root"
  PATH="$shim_dir:$PATH" "$shim_dir/rustfmt" --check crates/slskr/src/main.rs 2>&1
)"
rustfmt_status=$?
cargo_result="$(
  cd "$repo_root"
  PATH="$shim_dir:$PATH" "$shim_dir/cargo" fmt --all --check 2>&1
)"
cargo_status=$?
set -e

if [[ "$rustfmt_status" -ne 2 ]] || [[ "$rustfmt_result" != *'Rust format guard: rustfmt --check is disabled'* ]]; then
  printf 'Rust tool shim test failed: raw rustfmt --check was not rejected before execution\n%s\n' "$rustfmt_result" >&2
  exit 1
fi
if [[ "$cargo_status" -ne 2 ]] || [[ "$cargo_result" != *'Rust build guard: cargo fmt is disabled'* ]]; then
  printf 'Rust tool shim test failed: raw cargo fmt was not rejected before execution\n%s\n' "$cargo_result" >&2
  exit 1
fi

printf 'Rust tool shim tests passed\n'
