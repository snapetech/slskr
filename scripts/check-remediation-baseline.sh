#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

upstream_repo="${SLSKR_UPSTREAM_GIT_REPO:-$repo_root/../slskdn}"
slskd_ref="${SLSKR_SLSKD_REF:-16e5d86ec9a91120f3ef40b85cb22036566b788a}"
slskdn_ref="${SLSKR_SLSKDN_REF:-65a14a8b821de4df4ab7ef3ab3b156d7206837a3}"
reference_dir="$(mktemp -d "${TMPDIR:-/tmp}/slskr-remediation-references.XXXXXX")"
created_slskd=0
created_slskdn=0

export SLSKR_SLSKD_ROOT="${SLSKR_SLSKD_ROOT:-$reference_dir/slskd}"
export SLSKR_SLSKDN_ROOT="${SLSKR_SLSKDN_ROOT:-$reference_dir/slskdn}"

materialize_reference() {
  local root="$1"
  local ref="$2"
  local created_variable="$3"
  if [[ -d "$root/.git" || -f "$root/.git" ]]; then
    local actual
    actual="$(git -C "$root" rev-parse HEAD)"
    if [[ "$actual" != "$ref" ]]; then
      printf 'Frozen reference %s is at %s, expected %s\n' "$root" "$actual" "$ref" >&2
      exit 1
    fi
    return
  fi
  mkdir -p "$(dirname "$root")"
  git -C "$upstream_repo" worktree add --detach "$root" "$ref" >/dev/null
  printf -v "$created_variable" '%s' 1
}

cleanup_references() {
  if [[ "$created_slskd" == 1 ]]; then
    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKD_ROOT" >/dev/null 2>&1 || true
  fi
  if [[ "$created_slskdn" == 1 ]]; then
    git -C "$upstream_repo" worktree remove --force "$SLSKR_SLSKDN_ROOT" >/dev/null 2>&1 || true
  fi
  rm -rf "$reference_dir"
}
trap cleanup_references EXIT

materialize_reference "$SLSKR_SLSKD_ROOT" "$slskd_ref" created_slskd
materialize_reference "$SLSKR_SLSKDN_ROOT" "$slskdn_ref" created_slskdn

GATES=(
  scripts/check-endpoint-parity-drift.sh
  scripts/check-slskdn-controller-parity.sh
  scripts/check-controller-auth-profiles.sh
  scripts/check-controller-options-differential.sh
  scripts/check-diagnostics-memory-dump-differential.sh
  scripts/check-web-auth-credentials-differential.sh
  scripts/check-web-auth-disabled-differential.sh
  scripts/check-web-cors-differential.sh
  scripts/check-web-enforce-security-differential.sh
  scripts/check-web-no-auth-passthrough-differential.sh
  scripts/check-web-rate-limiting-differential.sh
  scripts/check-web-request-body-limit-differential.sh
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
