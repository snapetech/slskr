#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-002 BUG-012 BUG-014 BUG-016 BUG-023; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'workflow release policy check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

for expected in \
  'ACTIONLINT_VERSION: v' \
  'SLSKR_SECURITY_SCANS_REQUIRED:' \
  'SLSKR_SEMGREP_IMAGE: semgrep/semgrep:' \
  'SLSKR_TRIVY_IMAGE: aquasec/trivy:' \
  'concurrency:' \
  'attest-build-provenance@' \
  'attestations: write' \
  'id-token: write'; do
  if ! rg -n -F "$expected" .github/workflows >/dev/null; then
    printf 'workflow release policy check failed: expected workflow hardening token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n -F 'scripts/run-security-scans.sh' .github/workflows scripts/run-release-gate.sh >/dev/null; then
  printf 'workflow release policy check failed: required security scan runner is not wired into CI/release gates\n' >&2
  status=1
fi

if rg -n 'go install "github.com/rhysd/actionlint/cmd/actionlint@latest"|go install github.com/rhysd/actionlint/cmd/actionlint@latest' .github/workflows; then
  printf 'workflow release policy check failed: actionlint install must stay pinned\n' >&2
  status=1
fi

if ! rg -n -F "release-v*" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release-v tag trigger was not found\n' >&2
  status=1
fi

for expected in \
  'tag_pattern=' \
  'release-v<semver>' \
  "startsWith(github.ref, 'refs/tags/release-v')" \
  'version="${GITHUB_REF_NAME#release-}"'; do
  if ! rg -n -F "$expected" .github/workflows/release.yml >/dev/null; then
    printf 'workflow release policy check failed: release tag policy token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n -F 'release-v<semver>' docs/release.md >/dev/null; then
  printf 'workflow release policy check failed: release docs must document release-v<semver>\n' >&2
  status=1
fi

if rg -n -F "'release-*'" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: broad release-* tag trigger must not return\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'workflow release policy check passed\n'
