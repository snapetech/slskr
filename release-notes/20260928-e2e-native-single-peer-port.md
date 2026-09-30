---
category: fixed
audience: operators
area: test-harness
action: none
breaking: false
---
Local E2E nodes now share one loopback peer port across TCP, DHT, and QUIC transports. Failed node starts remain owned by harness cleanup; stopping a live child waits for its termination and clears the force-kill timer.
