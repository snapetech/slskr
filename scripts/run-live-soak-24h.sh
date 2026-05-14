#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${SLSKR_SOAK_CREDENTIAL_FILE:-$repo_root/.secrets/live-soak-account.env}"
log_file="${1:-$repo_root/target/live-soak/live-soak-24h-$(date +%Y%m%d-%H%M%S).log}"

mkdir -p "$(dirname "$log_file")"

set -a
# shellcheck disable=SC1090
source "$credential_file"
set +a

export SLSK_USERNAME="${SLSK_USERNAME:-${SLSKR_SOAK_USERNAME:-${SLSK_INTEGRATION_USERNAME:?missing soak username}}}"
export SLSK_PASSWORD="${SLSK_PASSWORD:-${SLSKR_SOAK_PASSWORD:-${SLSK_INTEGRATION_PASSWORD:?missing soak password}}}"
export SLSK_SOAK_SECONDS="${SLSK_SOAK_SECONDS:-86400}"
export SLSK_SOAK_MAX_EVENTS="${SLSK_SOAK_MAX_EVENTS:-200000}"
export SLSK_SOAK_PING_SECONDS="${SLSK_SOAK_PING_SECONDS:-300}"
export SLSK_SOAK_ACTIVE_PROBES="${SLSK_SOAK_ACTIVE_PROBES:-1}"
export SLSK_SOAK_DEFAULT_SEARCH="${SLSK_SOAK_DEFAULT_SEARCH:-1}"
export SLSK_SOAK_SEARCH_INTERVAL_SECONDS="${SLSK_SOAK_SEARCH_INTERVAL_SECONDS:-900}"
export SLSK_SOAK_OBFUSCATED_LISTENER_BIND="${SLSK_SOAK_OBFUSCATED_LISTENER_BIND:-0.0.0.0:0}"

cd "$repo_root"

cargo build -q -p slskr
slskr_bin="$repo_root/target/debug/slskr"

{
    printf '[slskr-live-soak start=%s]\n' "$(date -Is)"
    "$slskr_bin" soak live
    status=$?
    printf '[slskr-live-soak exit=%s at %s]\n' "$status" "$(date -Is)"
    exit "$status"
} >>"$log_file" 2>&1
