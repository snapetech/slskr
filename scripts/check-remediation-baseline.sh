#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

GATES=(
  scripts/check-endpoint-parity-drift.sh
  scripts/check-browser-token-persistence.sh
  scripts/check-unsafe-blank-opens.sh
  scripts/check-websocket-auth-coverage.sh
  scripts/check-csp-policy.sh
  scripts/check-webhook-outbound-policy.sh
  scripts/check-rate-limit-proxy-policy.sh
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
  scripts/check-shell-script-hygiene.sh
  scripts/check-kubernetes-public-posture.sh
  scripts/check-compatibility-noop-documentation.sh
  scripts/check-remediation-script-registry.sh
)

run_gate() {
  local gate="$1"
  printf '\n==> %s\n' "$gate"
  "$gate"
}

for gate in "${GATES[@]}"; do
  run_gate "$gate"
done

printf '\nRemediation baseline passed.\n'
