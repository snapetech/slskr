# slskR VPN host adapter contract

The optional host adapter exposes the compatibility surface consumed by the
daemon's VPN integration. It stores one mapping per `pfN.env` file under
`/var/lib/slskr-vpn` and serves:

- `GET /v1/openvpn/portforwarded`
- `GET /v1/openvpn/status`
- `GET /v1/portforward`
- `GET /v1/publicip/ip`
- `GET /v1/slskr/portforwards`

The adapter supports static provider mappings and can read the daemon's
authenticated `/api/v0/application` response. `split` installs a Linux UID
policy route with a blackhole fallback; `renew-ingress` refreshes the mapping
without tearing down existing state; `cleanup-ingress` removes state; and
`verify`/`watchdog` check NAT-PMP renewal-unit health and, for WireGuard
namespace forwards, reject missing or stale handshakes before recovering the
ingress unit.

The optional self-hosted relay adds `relay-apply` for bounded Linux forwarding
and traffic shaping, `relay-api` for its authenticated read-only status API,
and `relay-run` for both operations. Relay status requires an independent key
file and never exposes daemon credentials, arbitrary forwarding, SOCKS, or HTTP
proxy behavior. It serves `/v1/slskr/relay` plus the compatible port-forward
and public-IP endpoints; a stale tunnel reports an empty public IP.
