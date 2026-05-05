#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required="${SLSKR_SECURITY_SCANS_REQUIRED:-0}"
semgrep_image="${SLSKR_SEMGREP_IMAGE:-semgrep/semgrep:1.70.0}"
trivy_image="${SLSKR_TRIVY_IMAGE:-aquasec/trivy:0.50.1}"
status=0

section() {
  printf '\n==> %s\n' "$1"
}

run_or_record() {
  local label="$1"
  shift
  section "$label"
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  if "$@"; then
    return
  fi
  if [[ "$required" == "1" ]]; then
    printf '%s failed in required security-scan mode\n' "$label" >&2
    status=1
  else
    printf '%s failed; continuing because local security scans are optional by default\n' "$label" >&2
  fi
}

run_semgrep() {
  if command -v semgrep >/dev/null 2>&1; then
    run_or_record "Semgrep security scan" semgrep scan --config auto --error
    return
  fi

  if command -v docker >/dev/null 2>&1; then
    run_or_record "Semgrep security scan" docker run --rm \
      --user "$(id -u):$(id -g)" \
      -e HOME=/tmp \
      -v "$repo_root:/src" \
      -w /src \
      "$semgrep_image" \
      semgrep scan --config auto --error
    return
  fi

  section "Semgrep security scan"
  if [[ "$required" == "1" ]]; then
    printf 'semgrep security scan failed: semgrep and docker are unavailable\n' >&2
    status=1
  else
    printf 'semgrep and docker are unavailable; skipping optional local check.\n'
  fi
}

run_trivy() {
  if command -v trivy >/dev/null 2>&1; then
    run_or_record "Trivy filesystem scan" trivy fs --severity HIGH,CRITICAL --exit-code 1 --ignore-unfixed .
    return
  fi

  if command -v docker >/dev/null 2>&1; then
    run_or_record "Trivy filesystem scan" docker run --rm \
      --user "$(id -u):$(id -g)" \
      -e HOME=/tmp \
      -v "$repo_root:/src" \
      -w /src \
      "$trivy_image" \
      fs --severity HIGH,CRITICAL --exit-code 1 --ignore-unfixed .
    return
  fi

  section "Trivy filesystem scan"
  if [[ "$required" == "1" ]]; then
    printf 'trivy filesystem scan failed: trivy and docker are unavailable\n' >&2
    status=1
  else
    printf 'trivy and docker are unavailable; skipping optional local check.\n'
  fi
}

run_semgrep
run_trivy

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf '\nSecurity scans completed.\n'
