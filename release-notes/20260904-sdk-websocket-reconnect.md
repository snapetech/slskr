---
category: fixed
audience: users
area: sdk-websocket
action: none
breaking: false
---

Python and Go SDK websocket clients now reconnect after unexpected disconnects with bounded exponential backoff and restore their subscriptions automatically.
