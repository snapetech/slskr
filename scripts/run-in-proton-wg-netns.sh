#!/usr/bin/env bash
set -euo pipefail

if (( $# < 3 )); then
    echo "usage: $0 <namespace> <wireguard-conf> <command> [args...]" >&2
    exit 2
fi

namespace="$1"
config="$2"
shift 2
key_file=""

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_user="$(id -un)"
host_veth="v-${namespace}h"
ns_veth="v-${namespace}n"
wg_name="wg0"
host_ip="${SLSKR_NETNS_HOST_IP:-10.213.0.1}"
ns_ip="${SLSKR_NETNS_IP:-10.213.0.2}"
subnet="${SLSKR_NETNS_SUBNET:-10.213.0.0/24}"
gateway="${SLSKR_NETNS_GATEWAY:-10.2.0.1}"

cleanup() {
    if [[ -n "$key_file" ]]; then
        rm -f "$key_file"
    fi
    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
    sudo ip netns del "$namespace" 2>/dev/null || true
    sudo ip link del "$host_veth" 2>/dev/null || true
    sudo iptables -t nat -D POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || true
    sudo iptables -D FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || true
    sudo iptables -D FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
}

extract_first_value() {
    local key="$1"
    awk -v key="$key" '
        $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
            value=$0
            sub("^[[:space:]]*" key "[[:space:]]*=[[:space:]]*", "", value)
            sub(/^[[:space:]]*/, "", value)
            sub(/[[:space:]]*$/, "", value)
            split(value, parts, ",")
            sub(/^[[:space:]]*/, "", parts[1])
            sub(/[[:space:]]*$/, "", parts[1])
            print parts[1]
            exit
        }
    ' "$config"
}

extract_endpoint_ip() {
    extract_first_value Endpoint | sed -E 's/^[[]?([^]]+)[]]?:[0-9]+$/\1/'
}

cleanup
trap cleanup EXIT

private_key="$(extract_first_value PrivateKey)"
address="$(extract_first_value Address)"
peer_public_key="$(extract_first_value PublicKey)"
endpoint="$(extract_first_value Endpoint)"
endpoint_ip="$(extract_endpoint_ip)"

sudo ip netns add "$namespace"
sudo ip link add "$host_veth" type veth peer name "$ns_veth"
sudo ip link set "$ns_veth" netns "$namespace"
sudo ip addr add "$host_ip/24" dev "$host_veth"
sudo ip link set "$host_veth" up
sudo ip netns exec "$namespace" ip addr add "$ns_ip/24" dev "$ns_veth"
sudo ip netns exec "$namespace" ip link set "$ns_veth" up
sudo ip netns exec "$namespace" ip link set lo up
sudo ip netns exec "$namespace" ip route add default via "$host_ip" dev "$ns_veth"

sudo sysctl -q net.ipv4.ip_forward=1
sudo iptables -t nat -C POSTROUTING -s "$subnet" -j MASQUERADE 2>/dev/null || \
    sudo iptables -t nat -A POSTROUTING -s "$subnet" -j MASQUERADE
sudo iptables -C FORWARD -i "$host_veth" -j ACCEPT 2>/dev/null || \
    sudo iptables -A FORWARD -i "$host_veth" -j ACCEPT
sudo iptables -C FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
    sudo iptables -A FORWARD -o "$host_veth" -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT

sudo ip netns exec "$namespace" ip link add "$wg_name" type wireguard
sudo ip netns exec "$namespace" ip addr add "$address" dev "$wg_name"
key_file="$(mktemp)"
chmod 600 "$key_file"
printf '%s\n' "$private_key" >"$key_file"
sudo ip netns exec "$namespace" wg set "$wg_name" private-key "$key_file" \
    peer "$peer_public_key" endpoint "$endpoint" allowed-ips 0.0.0.0/0 persistent-keepalive 25
sudo ip netns exec "$namespace" ip link set mtu 1420 up dev "$wg_name"
sudo ip netns exec "$namespace" ip route add "$endpoint_ip/32" via "$host_ip" dev "$ns_veth"
sudo ip netns exec "$namespace" ip route replace default dev "$wg_name"

# Give WireGuard a moment to handshake before running the command.
sleep 2
sudo -E ip netns exec "$namespace" runuser --preserve-environment -u "$run_user" -- "$@"
