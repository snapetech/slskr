---
category: fixed
audience: developers
area: sdk
action: none
breaking: false
---
The Python WebSocket client now preserves topic subscriptions when an older connection fails while reconnecting, so the next handshake restores the caller's desired subscriptions.
