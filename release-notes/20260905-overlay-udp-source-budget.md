---
category: security
audience: operators
area: mesh-gateway
action: none
breaking: false
---
Overlay UDP control message budgets are now scoped to source IP addresses, so rotating source ports cannot bypass per-peer admission limits or exhaust tracked-peer capacity.
