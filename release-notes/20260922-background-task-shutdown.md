---
category: fixed
audience: operators
area: daemon-lifecycle
action: none
breaking: false
---

Long-lived daemon supervisors, configured listeners, and lifecycle support
tasks are now registered and aborted/joined during graceful shutdown and
restart, reducing shutdown overlap with active background services.
