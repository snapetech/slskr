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
host_endpoint_route_added=0

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_user="$(id -un)"
host_veth="v-${namespace}h"
ns_veth="v-${namespace}n"
wg_name="wg0"
if [[ -n "${SLSKR_NETNS_HOST_IP:-}" || -n "${SLSKR_NETNS_IP:-}" || -n "${SLSKR_NETNS_SUBNET:-}" ]]; then
    host_ip="${SLSKR_NETNS_HOST_IP:-10.213.0.1}"
    ns_ip="${SLSKR_NETNS_IP:-10.213.0.2}"
    subnet="${SLSKR_NETNS_SUBNET:-10.213.0.0/24}"
else
    ns_octet="$((100 + ($(printf '%s' "$namespace" | cksum | awk '{ print $1 }') % 120)))"
    host_ip="10.${ns_octet}.0.1"
    ns_ip="10.${ns_octet}.0.2"
    subnet="10.${ns_octet}.0.0/24"
fi
gateway="${SLSKR_NETNS_GATEWAY:-10.2.0.1}"

cleanup() {
    if [[ -n "$key_file" ]]; then
        rm -f "$key_file"
    fi
    sudo ip netns pids "$namespace" 2>/dev/null | xargs -r sudo kill 2>/dev/null || true
    sudo ip netns del "$namespace" 2>/dev/null || true
    sudo ip link del "$host_veth" 2>/dev/null || true
    if [[ "$host_endpoint_route_added" == "1" && -n "${endpoint_ip:-}" ]]; then
        sudo ip route del "$endpoint_ip/32" 2>/dev/null || true
    fi
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

private_key="$(extract_first_value PrivateKey)"
address="$(extract_first_value Address)"
peer_public_key="$(extract_first_value PublicKey)"
endpoint="$(extract_first_value Endpoint)"
endpoint_ip="$(extract_endpoint_ip)"

install_host_endpoint_route() {
    if [[ "${SLSKR_NETNS_BYPASS_HOST_VPN:-1}" != "1" ]]; then
        return 0
    fi
    if ! [[ "$endpoint_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        return 0
    fi

    local default_line default_via default_dev
    default_line="$(ip route show table main default 0.0.0.0/0 | awk 'NR == 1 { print }')"
    default_via="$(awk '{ for (i = 1; i <= NF; i++) if ($i == "via") { print $(i + 1); exit } }' <<<"$default_line")"
    default_dev="$(awk '{ for (i = 1; i <= NF; i++) if ($i == "dev") { print $(i + 1); exit } }' <<<"$default_line")"
    if [[ -z "$default_dev" ]]; then
        return 0
    fi

    if [[ -n "$default_via" ]]; then
        sudo ip route replace "$endpoint_ip/32" via "$default_via" dev "$default_dev"
    else
        sudo ip route replace "$endpoint_ip/32" dev "$default_dev"
    fi
    host_endpoint_route_added=1
}

cleanup
trap cleanup EXIT
install_host_endpoint_route

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
for extra_route in ${SLSKR_NETNS_EXTRA_ROUTES:-}; do
    sudo ip netns exec "$namespace" ip route replace "$extra_route" via "$host_ip" dev "$ns_veth"
done

# Force an initial handshake before running the command.
sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
sleep "${SLSKR_NETNS_WG_SETTLE_SECONDS:-2}"
sudo -E ip netns exec "$namespace" runuser --preserve-environment -u "$run_user" -- "$@"
