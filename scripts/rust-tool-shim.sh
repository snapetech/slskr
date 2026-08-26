#!/usr/bin/env bash
set -euo pipefail

# This file is installed under the user's early PATH by
# scripts/install-rust-tool-shims.sh. It only changes behavior when a Rust
# tool is invoked for this repository; other workspaces continue to use the
# normal rustup toolchain binary.
tool_name="$(basename "$0")"
case "$tool_name" in
  cargo|rustc|rustfmt)
    ;;
  *)
    printf 'Rust tool shim: unsupported command name: %s\n' "$tool_name" >&2
    exit 2
    ;;
esac

shim_path="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || printf '%s' "${BASH_SOURCE[0]}")"
repo_root="$(cd "$(dirname "$shim_path")/.." && pwd)"

resolve_real_tool() {
  local candidate resolved path_entry
  if command -v rustup >/dev/null 2>&1; then
    candidate="$(rustup which "$tool_name" 2>/dev/null || true)"
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  fi

  local -a path_entries=()
  IFS=: read -r -a path_entries <<<"${PATH:-}"
  for path_entry in "${path_entries[@]}"; do
    [[ -n "$path_entry" ]] || path_entry='.'
    candidate="$path_entry/$tool_name"
    [[ -x "$candidate" ]] || continue
    resolved="$(readlink -f "$candidate" 2>/dev/null || printf '%s' "$candidate")"
    [[ "$resolved" == "$shim_path" ]] && continue
    printf '%s\n' "$candidate"
    return 0
  done
  return 1
}

real_tool="$(resolve_real_tool || true)"
if [[ -z "$real_tool" || ! -x "$real_tool" ]]; then
  printf 'Rust tool shim: unable to resolve the real %s binary\n' "$tool_name" >&2
  exit 127
fi

path_touches_repo() {
  local input_path resolved_path
  case "$1" in
    "$repo_root"|"$repo_root"/*)
      return 0
      ;;
  esac

  input_path="$1"
  if [[ "$input_path" == /* ]]; then
    resolved_path="$input_path"
  else
    resolved_path="$PWD/$input_path"
  fi
  if [[ -e "$resolved_path" || -L "$resolved_path" ]]; then
    resolved_path="$(readlink -f "$resolved_path" 2>/dev/null || printf '%s' "$resolved_path")"
    case "$resolved_path" in
      "$repo_root"|"$repo_root"/*)
        return 0
        ;;
    esac
  fi
  return 1
}

repo_command=0
case "$PWD" in
  "$repo_root"|"$repo_root"/*)
    repo_command=1
    ;;
esac
if [[ "$repo_command" -eq 0 ]]; then
  for argument in "$@"; do
    case "$argument" in
      --*=*)
        argument="${argument#*=}"
        ;;
      -*|+*)
        continue
        ;;
    esac
    if path_touches_repo "$argument"; then
      repo_command=1
      break
    fi
  done
fi

if [[ "$repo_command" -eq 0 ]]; then
  exec "$real_tool" "$@"
fi

case "$tool_name" in
  cargo)
    # with-build-guard resolves and executes the real Cargo binary, avoiding
    # recursion through this shim.
    exec "$repo_root/scripts/with-build-guard.sh" cargo "$@"
    ;;
  rustc)
    exec "$repo_root/scripts/with-build-guard.sh" "$real_tool" "$@"
    ;;
  rustfmt)
    exec "$repo_root/scripts/with-rustfmt-guard.sh" "$@"
    ;;
esac
