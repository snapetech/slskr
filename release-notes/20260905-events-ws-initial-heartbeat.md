---
category: changed
audience: operators
area: events-websocket
action: none
breaking: false
---
Event WebSocket heartbeats now wait one configured interval before the first ping, allowing newly connected clients to complete close and control-frame handshakes without an initial heartbeat race.
