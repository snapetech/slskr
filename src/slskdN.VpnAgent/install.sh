#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-install}"

if [ "${EUID}" -ne 0 ]; then
  echo "Run as root: sudo $0" >&2
  exit 1
fi
if [ "$MODE" = "check" ] || [ "$MODE" = "--check" ]; then
  exec /usr/local/bin/slskr-vpn-agent verify
fi
[ "$MODE" = "install" ] || { echo "Usage: $0 [install|check]" >&2; exit 64; }

for command in curl ip iptables jq python3 systemctl; do
  command -v "$command" >/dev/null || { echo "Missing required command: $command" >&2; exit 2; }
done

install -D -m 0755 "$ROOT/slskr-vpn-agent" /usr/local/libexec/slskr-vpn-agent
ln -sfn /usr/local/libexec/slskr-vpn-agent /usr/local/bin/slskr-vpn-agent
for unit in "$ROOT"/systemd/*; do
  install -D -m 0644 "$unit" "/etc/systemd/system/$(basename "$unit")"
done
install -d -m 0750 /var/lib/slskr-vpn /etc/slskr-vpn/static-forwards
systemctl daemon-reload
systemctl enable slskr-vpn-split.service slskr-vpn-compat.service slskr-vpn-ingress.service
systemctl enable slskr-vpn-ingress-renew.timer slskr-vpn-watchdog.timer
echo "Installed slskR VPN host adapter. Configure static forwards before starting ingress."
