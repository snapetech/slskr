#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

GATES=(
  scripts/check-endpoint-parity-drift.sh
  scripts/check-slskdn-controller-parity.sh
  scripts/check-controller-auth-profiles.sh
  scripts/check-controller-options-differential.sh
  scripts/check-browser-token-persistence.sh
  scripts/check-unsafe-blank-opens.sh
  scripts/check-websocket-auth-coverage.sh
  scripts/check-csp-policy.sh
  scripts/check-webhook-outbound-policy.sh
  scripts/check-rate-limit-proxy-policy.sh
  scripts/check-runtime-boundary-hardening.sh
  scripts/check-storage-listing-pressure.sh
  scripts/check-transfer-event-growth.sh
  scripts/check-workflow-release-policy.sh
  scripts/check-package-artifact-matrix.sh
  scripts/check-rust-dependency-hygiene.sh
  scripts/check-release-version-metadata.sh
  scripts/check-secret-scanning.sh
  scripts/check-python-client-quality.sh
  scripts/check-client-sdk-gates.sh
  scripts/check-audit-tooling.sh
  scripts/check-rust-module-hygiene.sh
  scripts/check-dev-tooling.sh
  scripts/check-openapi-docs-drift.sh
  scripts/check-docs-freshness.sh
  scripts/check-council-loop.sh
  scripts/check-bug-council-all-phases.sh
  scripts/check-council-negative-space.sh
  scripts/check-council-active-backlog.sh
  scripts/check-council-inventory-closure.sh
  scripts/check-council-sweep-counts.sh
  scripts/check-rust-protocol-taint-lens.sh
  scripts/check-rust-protocol-adversarial-corpus.sh
  scripts/check-shell-script-hygiene.sh
  scripts/check-kubernetes-public-posture.sh
  scripts/check-compatibility-noop-documentation.sh
  scripts/check-local-identity-leaks.sh
  scripts/check-upstream-parity-classification.sh
  scripts/check-remediation-script-registry.sh
)

run_gate() {
  local gate="$1"
  printf '\n==> %s\n' "$gate"
  "$gate"
}

if [[ ! -f docs/dev/bug-council-active-backlog.md ]]; then
  printf 'Missing council active backlog: docs/dev/bug-council-active-backlog.md\n' >&2
  exit 1
fi

if ! tr '\n' ' ' < docs/dev/council-bughunt-playbook.md | rg -q -F 'Do not mention council, bughunt, scanners, agents, or other discovery tooling in commit messages'; then
  printf 'Missing commit wording policy in docs/dev/council-bughunt-playbook.md\n' >&2
  exit 1
fi

for gate in "${GATES[@]}"; do
  run_gate "$gate"
done

printf '\nRemediation baseline passed.\n'
