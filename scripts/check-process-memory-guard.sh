#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0
guard=scripts/with-process-memory-guard.sh
context_helper=scripts/process-memory-guard-active.sh
if [[ ! -x "$guard" ]]; then
  printf 'Process memory guard check failed: %s is missing or not executable\n' "$guard" >&2
  status=1
fi
if [[ ! -x "$context_helper" ]]; then
  printf 'Process memory guard check failed: %s is missing or not executable\n' "$context_helper" >&2
  status=1
fi
if ! rg -q '^hard_memory_kib=4194304$' "$guard"; then
  printf 'Process memory guard check failed: hard resident/virtual ceiling must be 4 GiB\n' >&2
  status=1
fi
if ! rg -q 'MemoryMax=' "$guard"; then
  printf 'Process memory guard check failed: systemd cgroup memory ceiling is missing\n' >&2
  status=1
fi
if ! rg -q 'MemorySwapMax=0' "$guard"; then
  printf 'Process memory guard check failed: guarded commands must not evade the RAM ceiling through swap\n' >&2
  status=1
fi
if ! rg -q 'ulimit -v' "$guard"; then
  printf 'Process memory guard check failed: portable virtual-memory fallback is missing\n' >&2
  status=1
fi
if ! rg -q 'process_guard_context_active=' "$guard"; then
  printf 'Process memory guard check failed: externally supplied nesting markers must be context-validated\n' >&2
  status=1
fi
if ! rg -q -- '--working-directory="\$repo_root"' "$guard"; then
  printf 'Process memory guard check failed: systemd units must preserve the repository working directory\n' >&2
  status=1
fi
if ! rg -q 'with-process-memory-guard\.sh' scripts/audit-parity-manifest.py; then
  printf 'Process memory guard check failed: parity manifest browser/build subprocesses are unguarded\n' >&2
  status=1
fi
if ! rg -q 'command = guarded_process_command\(command, cwd\)' scripts/audit-parity-manifest.py; then
  printf 'Process memory guard check failed: parity manifest Node inventory commands are unguarded\n' >&2
  status=1
fi
if ! rg -q 'with-process-memory-guard\.sh' scripts/run-release-gate.sh; then
  printf 'Process memory guard check failed: release-gate Node subprocesses are unguarded\n' >&2
  status=1
fi

for runner in \
  scripts/check-client-sdk-gates.sh \
  scripts/check-endpoint-parity-drift.sh \
  scripts/check-web-audit.sh \
  scripts/check-web-rate-limiting-differential.sh; do
  if ! rg -q 'process-memory-guard-active\.sh' "$runner"; then
    printf 'Process memory guard check failed: heavy runner is unguarded: %s\n' "$runner" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'Process memory guard static check passed\n'
