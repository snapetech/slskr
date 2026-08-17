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
policy route with a blackhole fallback; `cleanup-ingress` removes state, and
`watchdog` verifies the daemon health endpoint before restarting the ingress
unit after repeated failures.
