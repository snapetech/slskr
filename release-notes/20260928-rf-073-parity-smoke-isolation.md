---
category: security
audience: operators
area: release
action: none
breaking: false
---
The Live Parity slskd API smoke now installs its pinned client into unique temporary state instead of a restored Cargo target cache, uses isolated config, and binds its shared native peer TCP/UDP listener to an available loopback port. The loopback-only smoke disables its separate HTTPS API listener. Retained artifacts include only the passing API call summary; private mesh keys stay in temporary state outside the artifact tree and are deleted when the smoke exits.
