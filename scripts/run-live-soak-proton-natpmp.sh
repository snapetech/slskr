#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${SLSKR_SOAK_CREDENTIAL_FILE:-$repo_root/.secrets/live-soak-account.env}"
log_file="${1:-$repo_root/target/live-soak/live-soak-proton-natpmp-$(date +%Y%m%d-%H%M%S).log}"

gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
lifetime="${PROTON_NATPMP_LIFETIME:-60}"
renew_seconds="${PROTON_NATPMP_RENEW_SECONDS:-45}"
listen_port="${SLSK_LISTEN_PORT:-2234}"
obfuscated_port="${SLSK_SOAK_OBFUSCATED_LISTEN_PORT:-2235}"

mkdir -p "$(dirname "$log_file")"

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "missing required command: $1" >&2
        exit 127
    }
}

claim_tcp_port() {
    local private_port="$1"
    local output
    output="$(natpmpc -g "$gateway" -a 0 "$private_port" tcp "$lifetime")"
    printf '%s\n' "$output" >&2
    awk '
        /Mapped public port/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "port") {
                    print $(i + 1)
                    exit
                }
            }
        }
    ' <<<"$output"
}

renew_loop() {
    while true; do
        date -Is
        natpmpc -g "$gateway" -a "$advertised_port" "$listen_port" tcp "$lifetime"
        natpmpc -g "$gateway" -a "$obfuscated_advertised_port" "$obfuscated_port" tcp "$lifetime"
        sleep "$renew_seconds"
    done
}

require_command natpmpc
require_command cargo

set -a
# shellcheck disable=SC1090
source "$credential_file"
set +a

export SLSK_USERNAME="${SLSK_USERNAME:-${SLSKR_SOAK_USERNAME:?missing soak username}}"
export SLSK_PASSWORD="${SLSK_PASSWORD:-${SLSKR_SOAK_PASSWORD:?missing soak password}}"
export SLSK_LISTEN_PORT="$listen_port"
export SLSK_SOAK_LISTENER_BIND="${SLSK_SOAK_LISTENER_BIND:-0.0.0.0:$listen_port}"
export SLSK_SOAK_OBFUSCATED_LISTENER_BIND="${SLSK_SOAK_OBFUSCATED_LISTENER_BIND:-0.0.0.0:$obfuscated_port}"
export SLSK_SOAK_SECONDS="${SLSK_SOAK_SECONDS:-86400}"
export SLSK_SOAK_MAX_EVENTS="${SLSK_SOAK_MAX_EVENTS:-200000}"
export SLSK_SOAK_PING_SECONDS="${SLSK_SOAK_PING_SECONDS:-300}"

cd "$repo_root"

{
    printf '[slskr-proton-natpmp-soak start=%s gateway=%s listen_port=%s obfuscated_port=%s]\n' \
        "$(date -Is)" "$gateway" "$listen_port" "$obfuscated_port"

    advertised_port="$(claim_tcp_port "$listen_port")"
    obfuscated_advertised_port="$(claim_tcp_port "$obfuscated_port")"

    if [[ -z "$advertised_port" || -z "$obfuscated_advertised_port" ]]; then
        echo "failed to claim Proton NAT-PMP public ports"
        exit 1
    fi

    if [[ "${SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL:-0}" == "1" ]]; then
        export SLSK_SOAK_ADVERTISED_PORT="$listen_port"
    else
        export SLSK_SOAK_ADVERTISED_PORT="$advertised_port"
    fi
    export SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT="$obfuscated_advertised_port"
    printf '[slskr-proton-natpmp-soak mapped regular_public_port=%s advertised_port=%s obfuscated_advertised_port=%s]\n' \
        "$advertised_port" "$SLSK_SOAK_ADVERTISED_PORT" "$obfuscated_advertised_port"

    renew_loop &
    renew_pid=$!
    trap 'kill "$renew_pid" 2>/dev/null || true' EXIT

    cargo run -q -p slskr -- soak live
    status=$?
    printf '[slskr-proton-natpmp-soak exit=%s at %s]\n' "$status" "$(date -Is)"
    exit "$status"
} >>"$log_file" 2>&1
