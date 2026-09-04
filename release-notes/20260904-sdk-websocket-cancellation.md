---
category: fixed
audience: users, operators
area: sdk-websocket-cancellation
action: none
breaking: false
---
The Go event-feed client now cancels an active WebSocket dial and closes a socket that is still being initialized when disconnect is requested, so shutdown does not wait for the full connection deadline.
