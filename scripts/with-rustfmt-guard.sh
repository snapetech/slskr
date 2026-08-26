#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  printf 'usage: %s <rustfmt arguments...>\n' "${BASH_SOURCE[0]}" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# rustfmt's `--check` mode constructs a complete diff in memory. That is the
# exact path that previously attempted a host-sized allocation for main.rs.
# Formatting checks in this repository use --emit stdout plus cmp instead, so
# reject the dangerous mode before rustfmt starts.
for argument in "$@"; do
  if [[ "$argument" == "--check" ]]; then
    printf 'Rust format guard: rustfmt --check is disabled; use scripts/check-rust-format.sh\n' >&2
    exit 2
  fi
done

# Resolve the toolchain binary without going back through the workstation shim
# installed by scripts/install-rust-tool-shims.sh. The formatter gets a
# formatter-specific 1 GiB resident/virtual-memory ceiling, lower than both
# the general process guard and the Rust compiler guard.
rustfmt_binary=""
if command -v rustup >/dev/null 2>&1; then
  rustfmt_binary="$(rustup which rustfmt 2>/dev/null || true)"
fi
if [[ -z "$rustfmt_binary" ]]; then
  rustfmt_binary="$(command -v rustfmt || true)"
fi
if [[ -z "$rustfmt_binary" || ! -x "$rustfmt_binary" ]]; then
  printf 'Rust format guard: unable to resolve the real rustfmt binary\n' >&2
  exit 127
fi
resolved_rustfmt_binary="$(readlink -f "$rustfmt_binary" 2>/dev/null || printf '%s' "$rustfmt_binary")"
if [[ "$resolved_rustfmt_binary" == "$repo_root/scripts/rust-tool-shim.sh" ]]; then
  printf 'Rust format guard: rustfmt resolved to the repository shim instead of the toolchain binary\n' >&2
  exit 127
fi

SLSKR_PROCESS_MEMORY_MAX_KIB=1048576 \
  exec "$repo_root/scripts/with-process-memory-guard.sh" "$rustfmt_binary" "$@"
