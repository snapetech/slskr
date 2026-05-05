#!/usr/bin/env bash
set -euo pipefail

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
  "$@"
}

run_step "Public posture check" scripts/check-public-posture.sh
run_step "Shell syntax check" bash -n scripts/*.sh
run_step "Rust formatting" cargo fmt --all --check
run_step "Rust clippy" cargo clippy --workspace --all-targets -- -D warnings
run_step "Rust wasm web check" cargo check -p slskr-web --target wasm32-unknown-unknown
run_step "Rust tests" cargo test --workspace

if cargo audit --version >/dev/null 2>&1; then
  run_step "RustSec audit" cargo audit
else
  printf '\n==> RustSec audit\n'
  printf 'cargo-audit is not installed; skipping local advisory scan.\n'
fi

run_step "Rust package check" scripts/check-release-package.sh

if [[ "${SLSKR_RUN_SLSKD_API_COMPAT_SMOKE:-0}" == "1" ]]; then
  run_step "slskd API compatibility smoke" scripts/run-slskd-api-compat-smoke.sh
else
  printf '\n==> slskd API compatibility smoke\n'
  printf 'Set SLSKR_RUN_SLSKD_API_COMPAT_SMOKE=1 to run the live slskd_api automation-client smoke.\n'
fi

run_step "Install web dependencies" npm --prefix web ci
run_step "Web advisory audit" npm --prefix web audit --audit-level=moderate
run_step "Web lint" npm --prefix web run lint
run_step "Web tests" npm --prefix web test
run_step "Build web" npm --prefix web run build
run_step "Verify web build output" node web/scripts/verify-build-output.mjs
run_step "Smoke web subpath build" node web/scripts/smoke-subpath-build.mjs

run_step "Install dashboard dependencies" npm --prefix dashboard ci
run_step "Dashboard advisory audit" npm --prefix dashboard audit --audit-level=moderate
run_step "Dashboard type check" npm --prefix dashboard run type-check
run_step "Dashboard lint" npm --prefix dashboard run lint
run_step "Dashboard tests" npm --prefix dashboard test
run_step "Build dashboard" npm --prefix dashboard run build

run_step "Install TypeScript client dependencies" npm --prefix client-ts ci --ignore-scripts
run_step "TypeScript client advisory audit" npm --prefix client-ts audit --audit-level=moderate
run_step "TypeScript client lint" npm --prefix client-ts run lint
run_step "TypeScript client build" npm --prefix client-ts run build

printf '\nRelease gate passed.\n'
