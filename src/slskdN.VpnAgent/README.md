# slskR VPN host adapter

This optional Linux bundle connects the slskR VPN integration to a host VPN
interface. It does not start a VPN provider or claim ports through a provider
API. Operators supply a static `pf0.env` mapping or configure the daemon API
key so the adapter can read the current forwarded port.

Install with `sudo ./install.sh`, configure `/etc/slskr-vpn/static-forwards`,
then enable the supplied systemd units. The adapter owns the compatibility API,
state files, UID split route, ingress cleanup, and watchdog lifecycle.
