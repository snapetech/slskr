---
category: security
audience: operators
area: release
action: none
breaking: false
---
Live Parity now isolates the slskd API smoke from cached Cargo targets and per-user settings. The daemon uses one loopback peer TCP/UDP port, omits its unused HTTPS listener, and deletes generated mesh keys with its temporary state. Hosted artifacts retain only the passing API-call summary.
