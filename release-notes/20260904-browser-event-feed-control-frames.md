---
category: fixed
audience: users, operators
area: browser-event-feed
action: none
breaking: false
---
The TypeScript event-feed client no longer sends invalid JSON keepalive frames; browser WebSocket implementations now rely on the protocol-level ping/pong lifecycle, preventing the server from closing healthy event streams.
