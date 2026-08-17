---
category: changed
audience: operators
area: overlay-transport
action: none
breaking: false
---
Shared-port QUIC proxy admission now applies bounded global, per-prefix, and recent-attempt limits, preventing unauthenticated packet bursts from creating unbounded proxy state.
