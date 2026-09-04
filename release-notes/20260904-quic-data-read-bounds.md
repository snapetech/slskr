---
category: fixed
audience: operators
area: overlay-quic-data
action: none
breaking: false
---
Overlay QUIC data and control streams now abandon incomplete reads and blocked error responses after the configured transport deadline, preventing stalled peers from retaining handler capacity indefinitely.
