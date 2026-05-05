#!/usr/bin/env bash
set -euo pipefail

gateway="${PROTON_NATPMP_GATEWAY:-10.2.0.1}"
lifetime="${PROTON_NATPMP_LIFETIME:-60}"
renew_seconds="${PROTON_NATPMP_RENEW_SECONDS:-45}"
private_port="${SLSKR_NATPMP_PRIVATE_PORT:?missing SLSKR_NATPMP_PRIVATE_PORT}"
public_port_env="${SLSKR_NATPMP_PUBLIC_PORT_ENV:-SLSK_ADVERTISED_PORT}"

command -v natpmpc >/dev/null 2>&1 || {
  echo "missing required command: natpmpc" >&2
  exit 127
}

mapping="$(natpmpc -g "$gateway" -a 0 "$private_port" tcp "$lifetime")"
printf '%s\n' "$mapping" >&2
public_port="$(awk '
  /Mapped public port/ {
    for (i = 1; i <= NF; i++) {
      if ($i == "port") {
        print $(i + 1)
        exit
      }
    }
  }
' <<<"$mapping")"

if [[ -z "$public_port" ]]; then
  echo "failed to claim Proton NAT-PMP public port for local port $private_port" >&2
  exit 1
fi

renew() {
  while true; do
    natpmpc -g "$gateway" -a "$public_port" "$private_port" tcp "$lifetime" >/dev/null 2>&1 || true
    sleep "$renew_seconds"
  done
}

renew &
renew_pid=$!
trap 'kill "$renew_pid" 2>/dev/null || true' EXIT

export "$public_port_env=$public_port"
exec "$@"
