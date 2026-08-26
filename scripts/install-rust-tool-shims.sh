#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
shim_path="$repo_root/scripts/rust-tool-shim.sh"
if [[ ! -x "$shim_path" ]]; then
  printf 'Rust tool shim installer: %s is missing or not executable\n' "$shim_path" >&2
  exit 1
fi

user_home="$(getent passwd "$(id -u)" 2>/dev/null | cut -d: -f6 || true)"
if [[ -z "$user_home" ]]; then
  printf 'Rust tool shim installer: unable to resolve the current user home directory\n' >&2
  exit 1
fi
bin_dir="${SLSKR_RUST_TOOL_SHIM_BIN:-$user_home/.local/bin}"
mkdir -p "$bin_dir"

status=0
for tool_name in cargo rustc rustfmt; do
  target_path="$bin_dir/$tool_name"
  if [[ -e "$target_path" || -L "$target_path" ]]; then
    resolved_target="$(readlink -f "$target_path" 2>/dev/null || printf '%s' "$target_path")"
    resolved_shim="$(readlink -f "$shim_path" 2>/dev/null || printf '%s' "$shim_path")"
    if [[ "$resolved_target" == "$resolved_shim" ]]; then
      printf 'Rust tool shim already installed: %s\n' "$target_path"
    else
      printf 'Rust tool shim installer: refusing to replace existing %s\n' "$target_path" >&2
      status=1
    fi
    continue
  fi
  ln -s "$shim_path" "$target_path"
  printf 'Installed Rust tool shim: %s\n' "$target_path"
done

if [[ "$status" -ne 0 ]]; then
  printf 'Rust tool shim installer: resolve the reported conflicts, then rerun this script\n' >&2
  exit "$status"
fi

printf 'Rust tool shims installed for this checkout\n'
