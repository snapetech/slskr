#!/usr/bin/env bash
set -euo pipefail

mode="${1:?usage: probe-natpmp-mapping.sh <claim|renew|collision> <private-port>}"
private_port="${2:?usage: probe-natpmp-mapping.sh <claim|renew|collision> <private-port>}"
gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
natpmp_command="${NATPMP_COMMAND:-natpmpc}"
public_port=""
collision_private_port=""
collision_public_port=""

command -v "$natpmp_command" >/dev/null 2>&1 || {
    echo "missing required command: $natpmp_command" >&2
    exit 127
}

mapped_public_port() {
    awk '
        /Mapped public port/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "port") {
                    print $(i + 1)
                    exit
                }
            }
        }
    '
}

release_mappings() {
    if [[ -n "$collision_public_port" && -n "$collision_private_port" ]]; then
        "$natpmp_command" -g "$gateway" -a "$collision_public_port" \
            "$collision_private_port" tcp 0 >/dev/null 2>&1 || true
    fi
    if [[ -n "$public_port" ]]; then
        "$natpmp_command" -g "$gateway" -a "$public_port" \
            "$private_port" tcp 0 >/dev/null 2>&1 || true
    fi
}
trap release_mappings EXIT

claim_output="$("$natpmp_command" -g "$gateway" -a 0 "$private_port" tcp 30 2>&1)"
printf '%s\n' "$claim_output"
public_port="$(mapped_public_port <<<"$claim_output")"
[[ -n "$public_port" ]] || {
    echo "NAT-PMP claim did not return a public port" >&2
    exit 1
}

case "$mode" in
    claim)
        ;;
    renew)
        sleep 1
        renew_output="$("$natpmp_command" -g "$gateway" -a "$public_port" \
            "$private_port" tcp 60 2>&1)"
        printf '%s\n' "$renew_output"
        renewed_public_port="$(mapped_public_port <<<"$renew_output")"
        [[ "$renewed_public_port" == "$public_port" ]] || {
            echo "NAT-PMP renewal changed the public port" >&2
            exit 1
        }
        ;;
    collision)
        collision_private_port=$((private_port + 1000))
        collision_output="$("$natpmp_command" -g "$gateway" -a "$public_port" \
            "$collision_private_port" tcp 30 2>&1)"
        printf '%s\n' "$collision_output"
        collision_public_port="$(mapped_public_port <<<"$collision_output")"
        [[ -n "$collision_public_port" ]] || {
            echo "NAT-PMP collision request was not handled" >&2
            exit 1
        }
        ;;
    *)
        echo "unknown NAT-PMP probe mode: $mode" >&2
        exit 2
        ;;
esac
