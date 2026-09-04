---
category: fixed
audience: users, operators
area: event-feed-websocket
action: none
breaking: false
---
Browser and TypeScript event-feed clients now time out stalled websocket handshakes and recover through their normal reconnect lifecycle instead of leaving connection attempts pending indefinitely.
