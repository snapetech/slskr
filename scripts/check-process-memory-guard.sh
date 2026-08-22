#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0
guard=scripts/with-process-memory-guard.sh
if [[ ! -x "$guard" ]]; then
  printf 'Process memory guard check failed: %s is missing or not executable\n' "$guard" >&2
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
  scripts/run-slskd-cross-client-interop.sh \
  scripts/run-certification.sh \
  scripts/run-proton-public-matrix.sh \
  scripts/run-release-gate.sh \
  scripts/run-live-interop-matrix.sh \
  scripts/run-live-http-transfer-smoke.sh \
  scripts/run-cross-client-validation.sh \
  scripts/run-live-soak-24h.sh \
  scripts/run-live-soak-proton-natpmp.sh \
  scripts/check-controller-options-differential.sh \
  scripts/check-diagnostics-memory-dump-differential.sh \
  scripts/check-web-auth-credentials-differential.sh \
  scripts/check-web-auth-disabled-differential.sh \
  scripts/check-web-cors-differential.sh \
  scripts/check-web-enforce-security-differential.sh \
  scripts/check-web-no-auth-passthrough-differential.sh \
  scripts/check-web-rate-limiting-differential.sh \
  scripts/check-web-request-body-limit-differential.sh \
  scripts/check-slskdn-controller-parity.sh \
  scripts/build-release-archive.sh \
  scripts/check-client-sdk-gates.sh \
  scripts/check-endpoint-parity-drift.sh \
  scripts/check-web-audit.sh \
  scripts/run-universal-lifecycle-matrix.sh \
  scripts/generate-release-manifests.sh; do
  if ! rg -q 'with-process-memory-guard\.sh' "$runner"; then
    printf 'Process memory guard check failed: heavy runner is unguarded: %s\n' "$runner" >&2
    status=1
  fi
done

# A direct .NET host can retain a large managed heap even when Cargo itself is
# guarded. Fail closed if a future differential script adds a .NET launch
# without entering the repository process guard first.
while IFS= read -r -d '' dotnet_script; do
  if ! rg -q 'with-process-memory-guard\.sh' "$dotnet_script"; then
    printf 'Process memory guard check failed: direct .NET launcher is unguarded: %s\n' "$dotnet_script" >&2
    status=1
  fi
done < <(
  rg -l -0 --pcre2 '(^|[;&|(:`[:space:]])dotnet([[:space:]]|$|[`])' scripts --glob '*.sh' || true
)

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'Process memory guard static check passed\n'
