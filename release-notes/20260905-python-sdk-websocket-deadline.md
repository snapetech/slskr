---
category: fixed
audience: users
area: sdk
action: none
breaking: false
---
The Python SDK now keeps WebSocket connections usable when HTTP request deadlines are disabled with `timeout=0`, using its standard handshake deadline instead of rejecting the connection.
