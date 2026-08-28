#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target_dir="${SLSKR_RUST_WEB_DIST:-target/slskr-web}"
wasm_target="wasm32-unknown-unknown"

if ! rustup target list --installed | grep -qx "$wasm_target"; then
  echo "missing Rust target: $wasm_target" >&2
  echo "install it with: rustup target add $wasm_target" >&2
  exit 2
fi

wasm_bindgen_bin="$(command -v wasm-bindgen || true)"
if [[ -z "$wasm_bindgen_bin" && -x "$HOME/.cargo/bin/wasm-bindgen" ]]; then
  wasm_bindgen_bin="$HOME/.cargo/bin/wasm-bindgen"
fi

if [[ -z "$wasm_bindgen_bin" ]]; then
  echo "missing wasm-bindgen CLI" >&2
  echo "install it with: cargo install wasm-bindgen-cli" >&2
  exit 2
fi

cargo build -p slskr-web --release --target "$wasm_target"

rm -rf "$target_dir"
mkdir -p "$target_dir"
cp crates/slskr-web/static/index.html "$target_dir/"
cp crates/slskr-web/static/slskr_web_bootstrap.js "$target_dir/"
cp crates/slskr-web/static/styles.css "$target_dir/"

wasm_bindgen_target_dir="target/$wasm_target/release"
"$wasm_bindgen_bin" \
  --target web \
  --out-dir "$target_dir" \
  "$wasm_bindgen_target_dir/slskr_web.wasm"

printf '%s\n' "$target_dir"
