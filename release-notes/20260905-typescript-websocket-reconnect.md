---
category: fixed
audience: users, operators
area: typescript-sdk
action: none
breaking: false
---
The TypeScript WebSocket SDK now permits immediate reconnection after an intentional disconnect, even when the browser has not delivered the previous socket’s close event yet.
