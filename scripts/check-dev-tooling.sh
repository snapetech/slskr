#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'dev tooling check failed: required command is missing: %s\n' "$command_name" >&2
    status=1
  fi
}

note_optional_command() {
  local command_name="$1"
  local purpose="$2"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'dev tooling check note: optional command missing for %s: %s\n' "$purpose" "$command_name"
  fi
}

note_optional_cargo_subcommand() {
  local subcommand_name="$1"
  local purpose="$2"
  if ! cargo "$subcommand_name" --version >/dev/null 2>&1; then
    printf 'dev tooling check note: optional cargo subcommand missing for %s: cargo %s\n' "$purpose" "$subcommand_name"
  fi
}

for command_name in bash cargo node npm python3 rg git; do
  require_command "$command_name"
done

if ! command -v go >/dev/null 2>&1 && ! command -v docker >/dev/null 2>&1; then
  printf 'dev tooling check failed: Go client tests require go or docker fallback\n' >&2
  status=1
fi

note_optional_command shellcheck "shell lint"
note_optional_command actionlint "GitHub workflow lint"
note_optional_cargo_subcommand audit "RustSec advisory scan"
note_optional_command semgrep "local Semgrep scan"
note_optional_command trivy "local Trivy scan"
note_optional_command tmux "live soak sessions"
note_optional_command sudo "network namespace live tests"
note_optional_command ip "network namespace live tests"
note_optional_command wg "WireGuard live tests"
note_optional_command curl "live HTTP smoke tests"

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'dev tooling check passed\n'
