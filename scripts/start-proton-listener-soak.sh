#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 )); then
    echo "usage: $0 <wireguard-conf> <label>" >&2
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
config="$1"
label="$2"
session="${SLSKR_PROTON_SOAK_SESSION:-slskr-live-soak-proton}"
interface="${SLSKR_PROTON_INTERFACE:-slskrwg}"
active_config="$repo_root/.secrets/${interface}.conf"
log_file="$repo_root/target/live-soak/live-soak-proton-${label}-$(date +%Y%m%d-%H%M%S).log"

mkdir -p "$repo_root/target/live-soak" "$repo_root/.secrets"

tmux kill-session -t "$session" 2>/dev/null || true
sudo wg-quick down "$interface" 2>/dev/null || true
sudo ip link del "$interface" 2>/dev/null || true

# wg-quick DNS handling depends on local resolver setup; the soak does not need
# provider DNS, so drop DNS lines from the active copy.
awk '$1 != "DNS" { print }' "$config" >"$active_config"
chmod 600 "$active_config"

sudo wg-quick up "$active_config"

tmux new-session -d -s "$session" \
    "cd '$repo_root' && SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL='${SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL:-1}' scripts/run-live-soak-proton-natpmp.sh '$log_file'"

printf '%s\n' "$log_file"
