---
category: fixed
audience: users, operators
area: python-sdk
action: none
breaking: false
---
The Python WebSocket SDK now tolerates listener cleanup after registry teardown and listener removal during event dispatch, keeping shutdown and callback handling reliable.
