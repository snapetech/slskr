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
run_step "Rust tests" cargo test --workspace

if command -v cargo-audit >/dev/null 2>&1; then
  run_step "RustSec audit" cargo audit
else
  printf '\n==> RustSec audit\n'
  printf 'cargo-audit is not installed; skipping local advisory scan.\n'
fi

run_step "Rust package check" scripts/check-release-package.sh

run_step "Install web dependencies" npm --prefix web ci
run_step "Web tests" npm --prefix web test
run_step "Build web" npm --prefix web run build
run_step "Verify web build output" node web/scripts/verify-build-output.mjs
run_step "Smoke web subpath build" node web/scripts/smoke-subpath-build.mjs

printf '\nRelease gate passed.\n'
