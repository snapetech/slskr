#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

bash -n scripts/*.sh

status=0

if rg -n -g '!check-shell-script-hygiene.sh' 'curl .*\| *(bash|sh)|wget .*\| *(bash|sh)' scripts; then
  printf 'shell script hygiene check failed: network-to-shell execution matched above\n' >&2
  status=1
fi

if rg -n -g '!check-shell-script-hygiene.sh' 'rm -rf /|git reset --hard|git checkout -- \.' scripts; then
  printf 'shell script hygiene check failed: destructive command pattern matched above\n' >&2
  status=1
fi

if find scripts -maxdepth 1 -type f -name '*.sh' ! -perm -111 -print | rg .; then
  printf 'shell script hygiene check failed: shell scripts above are not executable\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'shell script hygiene check passed\n'
