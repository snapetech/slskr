---
category: fixed
audience: users, operators
area: typescript-sdk
action: none
breaking: false
---
The TypeScript WebSocket SDK now releases failed handshake sockets before rejecting, allowing immediate retry and keeping listener dispatch stable when callbacks change subscriptions.
