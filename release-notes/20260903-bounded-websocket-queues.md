---
category: fixed
audience: operators
area: websocket-lifecycle
action: none
breaking: false
---

Relay and SignalR WebSocket connections now use bounded inbound and outbound queues and clean up failed relay handshakes, preventing stalled clients from accumulating unbounded runtime state.
