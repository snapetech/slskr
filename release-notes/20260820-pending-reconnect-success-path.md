---
category: fixed
audience: operators
area: controller-compatibility
action: none
breaking: false
---
Successful server messages no longer mark the application as waiting for a
reconnect. The compatibility runtime now reports `pendingReconnect` only
when a reconnect-enabled server-message send actually fails.
