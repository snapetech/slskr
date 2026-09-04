---
category: fixed
audience: users, operators
area: sdk-websocket-lifecycle
action: none
breaking: false
---
Python and Go event-feed clients now honor bounded connection and write lifecycles, recover cleanly from stalled handshakes, and expire dead heartbeat sessions instead of retaining unusable connections indefinitely.
