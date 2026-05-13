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
namespace="${SLSKR_PROTON_NAMESPACE:-slskr-proton-listener}"
active_config="$repo_root/.secrets/${interface}.conf"
log_file="$repo_root/target/live-soak/live-soak-proton-${label}-$(date +%Y%m%d-%H%M%S).log"

mkdir -p "$repo_root/target/live-soak" "$repo_root/.secrets"

tmux kill-session -t "$session" 2>/dev/null || true
sudo wg-quick down "$interface" 2>/dev/null || true
sudo ip link del "$interface" 2>/dev/null || true
sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
sudo ip netns del "$namespace" 2>/dev/null || true

# The listener must not use host wg-quick: Proton configs are full-tunnel
# (AllowedIPs 0.0.0.0/0, ::/0), and wg-quick would route the whole host
# through the VPN. Keep a DNS-stripped active copy and run the soak inside an
# isolated network namespace instead.
tmp_config="$(mktemp)"
trap 'rm -f "$tmp_config"' EXIT
awk '$1 != "DNS" { print }' "$config" >"$tmp_config"
install -m 600 "$tmp_config" "$active_config"
chmod 600 "$active_config"

tmux new-session -d -s "$session" \
    "cd '$repo_root' && SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL='${SLSKR_PROTON_ADVERTISE_REGULAR_LOCAL:-1}' scripts/run-in-proton-wg-netns.sh '$namespace' '$active_config' scripts/run-live-soak-proton-natpmp.sh '$log_file'"

printf '%s\n' "$log_file"
