#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
pin_policy="docs/dev/github-actions-pin-policy.md"
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
  'actions/attest-build-provenance@v4' \
  'attestations: write' \
  'id-token: write'; do
  if ! rg -n -F "$expected" .github/workflows >/dev/null; then
    printf 'workflow release policy check failed: expected workflow hardening token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if [[ ! -f "$pin_policy" ]]; then
  printf 'workflow release policy check failed: GitHub Actions pin policy is missing: %s\n' "$pin_policy" >&2
  status=1
else
  while IFS= read -r line; do
    action="${line#*uses: }"
    action="${action%%#*}"
    action="${action%%[[:space:]]*}"

    if [[ "$action" == ./* || "$action" == docker://* ]]; then
      continue
    fi

    if [[ ! "$action" =~ @([0-9a-f]{40})$ ]]; then
      printf 'workflow release policy check failed: external action must be pinned to a 40-character commit SHA: %s\n' "$line" >&2
      status=1
      continue
    fi

    action_name="${action%@*}"
    action_sha="${action##*@}"
    if ! rg -n -F "| \`${action_name}\` |" "$pin_policy" >/dev/null; then
      printf 'workflow release policy check failed: pinned action is missing from policy ledger: %s\n' "$action_name" >&2
      status=1
    fi
    if ! rg -n -F "\`${action_sha}\`" "$pin_policy" >/dev/null; then
      printf 'workflow release policy check failed: pinned action SHA is missing from policy ledger: %s\n' "$action_sha" >&2
      status=1
    fi
  done < <(rg -n '^[[:space:]-]*uses:[[:space:]]+[^[:space:]#]+@[^[:space:]#]+' .github/workflows)
fi

if ! rg -n -F 'scripts/run-security-scans.sh' .github/workflows scripts/run-release-gate.sh >/dev/null; then
  printf 'workflow release policy check failed: required security scan runner is not wired into CI/release gates\n' >&2
  status=1
fi

for expected in \
  'name: Live Parity' \
  'workflow_dispatch:' \
  'schedule:' \
  'node scripts/audit-rust-web-ui.mjs' \
  'scripts/run-slskd-api-compat-smoke.sh' \
  'SLSKR_SLSKD_API_SMOKE_DIR: target/slskd-api-smoke' \
  'SLSKR_SLSKD_API_SMOKE_TOKEN:' \
  'target/ux-audit/**' \
  'target/slskd-api-smoke/**' \
  'Credentialed public live interop' \
  'SLSKR_LIVE_INTEROP_ENV: ${{ secrets.SLSKR_LIVE_INTEROP_ENV }}' \
  'scripts/run-live-interop-matrix.sh' \
  'target/live-interop/**' \
  'credentialed-live-interop.tsv'; do
  if ! rg -n -F "$expected" .github/workflows/live-parity.yml >/dev/null; then
    printf 'workflow release policy check failed: live parity workflow token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if rg -n 'go install "github.com/rhysd/actionlint/cmd/actionlint@latest"|go install github.com/rhysd/actionlint/cmd/actionlint@latest' .github/workflows; then
  printf 'workflow release policy check failed: actionlint install must stay pinned\n' >&2
  status=1
fi

if ! rg -n -F "release-v*" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release-v tag trigger was not found\n' >&2
  status=1
fi

if rg -n -F "workflow_dispatch:" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release workflow must only run from release-v tags\n' >&2
  status=1
fi

if rg -n -F "branches:" .github/workflows/ci.yml >/dev/null; then
  printf 'workflow release policy check failed: CI must not build on main pushes\n' >&2
  status=1
fi

for expected in \
  'tag_pattern=' \
  'release-v<semver>' \
  "startsWith(github.ref, 'refs/tags/release-v')" \
  'version="${GITHUB_REF_NAME#release-}"' \
  'DISCORD_RELEASE_WEBHOOK_URL' \
  'Announce Discord Release'; do
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
