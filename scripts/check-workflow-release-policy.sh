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
  'actions: read' \
  'concurrency:' \
  'actions/attest-build-provenance@v4' \
  'attestations: write' \
  'id-token: write'; do
  if ! rg -n -F -- "$expected" .github/workflows >/dev/null; then
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
  if ! rg -n -F -- "$expected" .github/workflows/live-parity.yml >/dev/null; then
    printf 'workflow release policy check failed: live parity workflow token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if rg -n 'go install "github.com/rhysd/actionlint/cmd/actionlint@latest"|go install github.com/rhysd/actionlint/cmd/actionlint@latest' .github/workflows; then
  printf 'workflow release policy check failed: actionlint install must stay pinned\n' >&2
  status=1
fi

for workflow in .github/workflows/ci.yml .github/workflows/release.yml; do
  if ! rg -n -F "go-version: '1.25.x'" "$workflow" >/dev/null; then
    printf 'workflow release policy check failed: actionlint v1.7.12 requires Go 1.25 or newer: %s\n' "$workflow" >&2
    status=1
  fi
done

if ! rg -n -F "release-v*" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release-v tag trigger was not found\n' >&2
  status=1
fi

if rg -n -F "workflow_dispatch:" .github/workflows/release.yml >/dev/null; then
  printf 'workflow release policy check failed: release workflow must only run from release-v tags\n' >&2
  status=1
fi

if rg -n -F "types: [published]" .github/workflows/release-publish.yml >/dev/null; then
  printf 'workflow release policy check failed: downstream package publish must be dispatched after release assets are uploaded\n' >&2
  status=1
fi

for expected in \
  'workflow_call:' \
  'Build and push (ghcr)' \
  'platforms: linux/amd64,linux/arm64' \
  'push: true' \
  'Verify pushed GHCR manifest' \
  'scripts/run-container-shutdown-smoke.sh'; do
  if ! rg -n -F -- "$expected" .github/workflows/release-publish.yml >/dev/null; then
    printf 'workflow release policy check failed: container publishing contract token missing: %s\n' "$expected" >&2
    status=1
  fi
done

for expected in \
  'uses: ./.github/workflows/release-publish.yml' \
  'AUR_SSH_KEY: ${{ secrets.AUR_SSH_KEY }}' \
  'needs: [release]' \
  'packages: write'; do
  if ! rg -n -F -- "$expected" .github/workflows/release.yml >/dev/null; then
    printf 'workflow release policy check failed: release must call the downstream publisher: %s\n' "$expected" >&2
    status=1
  fi
done

for dockerfile in Dockerfile packaging/docker/release.Dockerfile; do
  for expected in 'STOPSIGNAL SIGTERM' 'ENTRYPOINT ["slskr"]' 'CMD ["serve"]'; do
    if ! rg -n -F -- "$expected" "$dockerfile" >/dev/null; then
      printf 'workflow release policy check failed: container lifecycle token missing from %s: %s\n' "$dockerfile" "$expected" >&2
      status=1
    fi
  done
done

for expected in \
  'push:' \
  'branches:' \
  '- main' \
  'cargo audit'; do
  if ! rg -n -F -- "$expected" .github/workflows/ci.yml >/dev/null; then
    printf 'workflow release policy check failed: CI must run advisory coverage on main pushes; missing token: %s\n' "$expected" >&2
    status=1
  fi
done

ci_cancel_policy='cancel-in-progress: ${{ github.event_name == '\''pull_request'\'' }}'
if ! rg -n -F -- "$ci_cancel_policy" .github/workflows/ci.yml >/dev/null; then
  printf 'workflow release policy check failed: main CI pushes must not cancel superseded runs\n' >&2
  status=1
fi

for expected in \
  'scripts/submit-winget-release.ps1' \
  'WINGETCREATE_GITHUB_TOKEN'; do
  if ! rg -n -F -- "$expected" .github/workflows/release-publish.yml >/dev/null; then
    printf 'workflow release policy check failed: Winget submission token missing: %s\n' "$expected" >&2
    status=1
  fi
done

for expected in \
  'merge-upstream' \
  'git/refs/heads/' \
  'slskr-release-backup-' \
  'force = $true' \
  'wingetcreate submit'; do
  if ! rg -n -F -- "$expected" scripts/submit-winget-release.ps1 >/dev/null; then
    printf 'workflow release policy check failed: Winget fork recovery token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if rg -n 'runs-on:[[:space:]]*\[self-hosted|packer-windows' .github/workflows .github/WINDOWS_RUNNER.md >/dev/null; then
  printf 'workflow release policy check failed: current Windows workflows/docs must not refer to retired self-hosted runners\n' >&2
  status=1
fi

if ! rg -n -F 'cargo audit' .github/workflows/ci.yml .github/workflows/release.yml scripts/run-release-gate.sh >/dev/null; then
  printf 'workflow release policy check failed: RustSec audit must stay wired into CI and release gates\n' >&2
  status=1
fi

for expected in \
  'tag_pattern=' \
  'release-v<semver>' \
  'Require Main CI' \
  'for _ in {1..240}' \
  '--workflow CI' \
  '--branch main' \
  '--commit "$RELEASE_SHA"' \
  'needs: [version, main-ci]' \
  "startsWith(github.ref, 'refs/tags/release-v')" \
  'version="${GITHUB_REF_NAME#release-}"' \
  'DISCORD_RELEASE_WEBHOOK_URL' \
  'Announce Discord Release' \
  'scripts/release_notes.py' \
  'User-facing changes' \
  'RELEASE_BODY' \
  'slskr Releases' \
  'scripts/validate-changelog.sh'; do
  if ! rg -n -F -- "$expected" .github/workflows/release.yml >/dev/null; then
    printf 'workflow release policy check failed: release tag policy token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if ! rg -n -F 'release-v<semver>' docs/release.md >/dev/null; then
  printf 'workflow release policy check failed: release docs must document release-v<semver>\n' >&2
  status=1
fi

if ! rg -n -F '`macos-15-intel`' docs/release.md >/dev/null; then
  printf 'workflow release policy check failed: release docs must document the current macOS Intel runner\n' >&2
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
