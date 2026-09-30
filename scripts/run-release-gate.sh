#!/usr/bin/env bash
set -euo pipefail

# The release gate includes dependency installs, browser audits, Rust builds,
# and package builds. Keep only Node/frontend steps inside the process-memory
# guard; Cargo uses the workspace's normal build configuration.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

section() {
  printf '\n==> %s\n' "$1"
}

run_step() {
  local label="$1"
  shift
  section "$label"
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  case "${1##*/}" in
    node|nodejs|npm|npx)
      scripts/with-process-memory-guard.sh "$@"
      ;;
    *)
      "$@"
      ;;
  esac
}

run_optional_step() {
  local tool="$1"
  local label="$2"
  shift 2
  if command -v "$tool" >/dev/null 2>&1; then
    run_step "$label" "$@"
  else
    printf '\n==> %s\n' "$label"
    printf '%s is not installed; skipping optional local check.\n' "$tool"
  fi
}

run_step "Rust debug build" cargo build --locked -q -p slskr
run_step "Remediation baseline" scripts/check-remediation-baseline.sh
run_step "Public posture check" scripts/check-public-posture.sh
run_step "Changelog validation" scripts/validate-changelog.sh
run_step "Shell syntax check" bash -n scripts/*.sh
run_step "Release-note tooling tests" python3 scripts/test_release_notes.py
run_step "Benchmark comparison and SQLite profiler tests" python3 -m unittest -v scripts.test_compare_benchmark scripts.test_profile_sqlite
run_optional_step shellcheck "Shell lint" shellcheck \
  -x \
  -e SC1090,SC2016,SC2030,SC2031,SC2034,SC2100,SC2155,SC2206,SC2317,SC2329 \
  scripts/*.sh
run_optional_step actionlint "GitHub workflow lint" actionlint
run_step "Security scans" scripts/run-security-scans.sh
run_step "Rust formatting" scripts/check-rust-format.sh
run_step "Rust clippy" cargo clippy --locked --workspace --all-targets -- -D warnings
run_step "Rust wasm web check" cargo check --locked -p slskr-web --target wasm32-unknown-unknown
run_step "Rust tests" cargo test --locked --workspace

if cargo audit --version >/dev/null 2>&1; then
  run_step "RustSec audit" cargo audit
else
  printf '\n==> RustSec audit\n'
  printf 'cargo-audit is not installed; skipping local advisory scan.\n'
fi

run_step "Rust package check" scripts/check-release-package.sh
run_step "AUR package smoke" scripts/check-aur-package-smoke.sh
if [[ "${SLSKR_RUN_SLSKD_API_COMPAT_SMOKE:-0}" == "1" ]]; then
  run_step "slskd API compatibility smoke" scripts/run-slskd-api-compat-smoke.sh
else
  printf '\n==> slskd API compatibility smoke\n'
  printf 'Skipped by local policy; set SLSKR_RUN_SLSKD_API_COMPAT_SMOKE=1 for the live smoke. Release certification requires a retained scheduled live-parity artifact.\n'
fi

run_step "Install web dependencies" npm --prefix web ci
run_step "Rust web UI headless audit" node scripts/audit-rust-web-ui.mjs
run_step "Web advisory audit" bash scripts/check-web-audit.sh web
run_step "Web lint" npm --prefix web run lint
run_step "Web tests" npm --prefix web test
run_step "Web API and lifecycle coverage" npm --prefix web run test:api-lifecycle-coverage
run_step "Build web" npm --prefix web run build
run_step "Web bundle budget" npm --prefix web run test:bundle-budget
run_step "Verify web build output" node web/scripts/verify-build-output.mjs
run_step "Smoke web subpath build" node web/scripts/smoke-subpath-build.mjs

run_step "Install dashboard dependencies" npm --prefix dashboard ci
run_step "Dashboard advisory audit" bash scripts/check-web-audit.sh dashboard
run_step "Dashboard type check" npm --prefix dashboard run type-check
run_step "Dashboard lint" npm --prefix dashboard run lint
run_step "Dashboard tests and coverage threshold" npm --prefix dashboard run test:coverage
run_step "Build dashboard" npm --prefix dashboard run build
run_step "Dashboard bundle budget" npm --prefix dashboard run test:bundle-budget

printf '\nRelease gate passed.\n'
